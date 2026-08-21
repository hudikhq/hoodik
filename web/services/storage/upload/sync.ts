import Api, { ErrorResponse } from '../../api'
import * as cryptfns from '../../cryptfns'
import { utcStringFromLocal } from '../..'
import { MAX_UPLOAD_RETRIES } from '../../constants'
import { forgetUpload, putChunk, uploadChunkUrls } from './direct'
import * as logger from '!/logger'

import type { Query } from '../../api'
import type { AppFile, UploadAppFile } from '../../../types'

/**
 * Upload a single encrypted chunk of a file.
 *
 * Every upload the page performs itself goes through here — note saves, forks,
 * and the fallback for when the WASM upload worker is unavailable — so this is
 * where direct-versus-relayed is decided for all of them. The manifest is
 * fetched once for the file and held until it commits, so asking per chunk
 * costs one request, not one per chunk.
 *
 * Encryption happens before either branch. What differs is only where the
 * ciphertext is sent.
 *
 * @param api  Optional Api instance with a transfer token. If omitted, falls back to session auth.
 */
export async function uploadChunk(
  file: UploadAppFile,
  data: Uint8Array,
  chunk: number,
  attempt: number = 0,
  api?: Api
): Promise<UploadAppFile> {
  if (!file.key) {
    throw new Error(`File ${file.id} is missing key`)
  }

  const encrypted = await cryptfns.cipher.encrypt(file.cipher, data, file.key, chunk)

  const client = api || new Api().withRefresh()
  const directUrl = (await uploadChunkUrls(file, client))?.[chunk]
  if (directUrl) {
    logger.debug(
      'Direct',
      `Writing chunk ${chunk} / ${file.chunks} of ${file.name} (${encrypted.length} B) into the bucket`
    )

    try {
      const committed = await putChunk(file, chunk, encrypted, directUrl, client)

      return {
        ...file,
        ...(committed || {}),
        key: file.key,
        name: file.name,
        thumbnail: file.thumbnail,
        temporaryId: file.temporaryId,
        file: file.file,
        started_upload_at: file.started_upload_at || utcStringFromLocal()
      }
    } catch (err) {
      // Buckets shed load with transient errors that every S3 SDK retries,
      // and a URL can expire under a long-stalled queue. `putChunk` already
      // dropped the manifest, so a retry signs fresh URLs; a chunk that
      // keeps failing falls through to the relaying route below rather than
      // sinking the save.
      if (attempt < MAX_UPLOAD_RETRIES) {
        logger.warn(
          'Direct',
          `Failed writing chunk ${chunk} / ${file.chunks} of ${file.name} into the bucket, retrying...`,
          err
        )
        return uploadChunk(file, data, chunk, attempt + 1, api)
      }

      logger.warn(
        'Direct',
        `Chunk ${chunk} / ${file.chunks} of ${file.name} keeps failing directly, relaying instead...`,
        err
      )
    }
  }

  const checksum = await cryptfns.wasm.crc16_digest(encrypted)

  const query: Query = {
    chunk,
    checksum,
    checksum_function: 'crc16'
  }

  const headers = {
    'Content-Type': 'application/octet-stream'
  }

  try {
    logger.debug(
      'Sync',
      `Uploading chunk (${encrypted.length} B) ${chunk} / ${file.chunks} of ${file.file.name} - upload attempt ${attempt} (checksum: ${checksum})`
    )

    const response = await client.make<Uint8Array, AppFile>(
      'post',
      `/api/storage/${file.id}`,
      query,
      encrypted,
      headers
    )

    if (!response?.body) {
      throw new Error(`Missing response body`)
    }

    const uploaded = response.body

    return {
      ...uploaded,
      key: file.key,
      name: file.name,
      thumbnail: file.thumbnail,
      temporaryId: file.temporaryId,
      file: file.file,
      started_upload_at: file.started_upload_at || utcStringFromLocal()
    }
  } catch (err) {
    const error = err as ErrorResponse<Uint8Array>

    // If we get checksum error, most likely the data was corrupted during transfer
    // we wont retry indefinitely, but we will try a few times
    if (error.validation?.checksum && attempt < MAX_UPLOAD_RETRIES) {
      logger.warn(
        `Failed uploading chunk ${chunk} / ${file.chunks} of ${file.file.name}, failed checksum, retrying...`
      )
      return uploadChunk(file, data, chunk, attempt + 1, api)
    }

    // The chunk was already uploaded, so we can just return the file
    if (error.validation?.chunk === 'chunk_already_exists') {
      logger.warn(
        'Sync',
        `Failed uploading chunk ${chunk} / ${file.chunks} of ${file.name}, chunk already exist, skipping...`
      )
      return file
    }

    logger.error(
      'Sync',
      `Failed uploading chunk ${chunk} / ${file.chunks} of ${file.name}, either some unexpected error, or too many failed checksum tries, aborting...`,
      err
    )

    forgetUpload(file.id)

    throw err
  }
}
