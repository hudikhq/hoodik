import Api, { ErrorResponse } from '../../api'
import * as cryptfns from '../../cryptfns'
import { utcStringFromLocal } from '@/stores'
import { MAX_UPLOAD_RETRIES } from '../constants'

import type { Query } from '../../api'
import type { AppFile, UploadAppFile } from '../types'

/**
 * Upload a single file chunk
 */
export async function uploadChunk(
  file: UploadAppFile,
  data: Uint8Array,
  chunk: number,
  attempt: number = 0
): Promise<UploadAppFile> {
  if (!file.metadata?.key) {
    throw new Error(`File ${file.id} is missing key`)
  }

  // const encrypted = data
  const encrypted = cryptfns.aes.encrypt(data, file.metadata?.key)
  const checksum = cryptfns.sha256.digest(encrypted)

  // Data can be encrypted also on the server, but this method is less secure
  // const key_hex = cryptfns.uint8.toHex(file.metadata.key)
  const query: Query = {
    chunk,
    checksum
    // key_hex
  }

  const headers = {
    'Content-Type': 'application/octet-stream'
  }

  try {
    console.log(
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
      metadata: file.metadata,
      file: file.file,
      started_upload_at: file.started_upload_at || utcStringFromLocal()
    }
  } catch (err) {
    const error = err as ErrorResponse<Uint8Array>

    // If we get checksum error, most likely the data was corrupted during transfer
    // we wont retry indefinitely, but we will try a few times
    if (error.validation?.checksum && attempt < MAX_UPLOAD_RETRIES) {
      console.warn(
        `Failed uploading chunk ${chunk} / ${file.chunks} of ${file.file.name}, failed checksum, retrying...`
      )
      return uploadChunk(file, data, chunk, attempt + 1)
    }

    // The chunk was already uploaded, so we can just return the file
    if (error.validation?.chunk === 'chunk_already_exists') {
      console.warn(
        `Failed uploading chunk ${chunk} / ${file.chunks} of ${file.file.name}, chunk already exist, skipping...`
      )
      return file
    }

    console.error(
      `Failed uploading chunk ${chunk} / ${file.chunks} of ${file.file.name}, either some unexpected error, or too many failed checksum tries, aborting...`,
      err
    )

    throw err
  }
}
