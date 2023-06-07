import Api, { ErrorResponse } from '../../api'
import * as cryptfns from '../../cryptfns'
import { utcStringFromLocal } from '../..'
import { MAX_UPLOAD_RETRIES } from '../../constants'
import * as logger from '!/logger'

import type { Query } from '../../api'
import type { AppFile, UploadAppFile } from '../../../types'

/**
 * Upload a single file chunk
 *
 * This is a fallback version of upload and in case the upload worker
 * is not available, this method will be used instead.
 *
 * NOTICE: This is a less safe method of uploading since it assumes
 * that the missing worker means older browser is being used, so
 * it also assumes slower system is used and it offloads the encryption
 * process to the server. Which means it will send the encryption key
 * to server to encrypt the data before its being stored.
 *
 * The key is kept only in memory and is never saved anywhere on the server,
 * but still... If the connection is not secure it is possible the key could be
 * intercepted.
 */
export async function uploadChunk(
  file: UploadAppFile,
  data: Uint8Array,
  chunk: number,
  attempt: number = 0
): Promise<UploadAppFile> {
  if (!file.key) {
    throw new Error(`File ${file.id} is missing key`)
  }

  const encrypted = data
  // const encrypted = await cryptfns.aes.encrypt(data, file.key)
  // const checksum = await cryptfns.sha256.digest(encrypted)
  const checksum = await cryptfns.wasm.crc16_digest(encrypted)

  // Data can be encrypted also on the server, but this method is less secure
  const key_hex = cryptfns.uint8.toHex(file.key)
  const query: Query = {
    chunk,
    checksum,
    checksum_function: 'crc16',
    key_hex
  }

  const headers = {
    'Content-Type': 'application/octet-stream'
  }

  try {
    logger.debug(
      'Sync',
      `Uploading chunk (${encrypted.length} B) ${chunk} / ${file.chunks} of ${file.file.name} - upload attempt ${attempt} (checksum: ${checksum})`
    )

    const response = await Api.post<Uint8Array, AppFile>(
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
      return uploadChunk(file, data, chunk, attempt + 1)
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

    throw err
  }
}
