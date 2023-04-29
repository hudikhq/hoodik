import * as cryptfns from '@/stores/cryptfns'
import { utcStringFromLocal } from '@/stores'
import { MAX_UPLOAD_RETRIES } from '@/stores/storage/constants'

import type { AppFile, UploadAppFile, UploadChunkResponseMessage } from '@/types'
import type Api from '@/stores/api'
import type { ErrorResponse } from '@/stores/api'

/**
 * Upload a single chunk
 */
export async function uploadChunk(
  api: Api,
  file: UploadAppFile,
  data: Uint8Array,
  chunk: number,
  attempt: number
): Promise<UploadChunkResponseMessage> {
  try {
    if (!file.metadata?.key) {
      throw new Error(`File ${file.id} is missing key`)
    }

    const encrypted = cryptfns.aes.encrypt(data, file.metadata.key)
    const checksum = cryptfns.sha256.digest(encrypted)

    if (!encrypted.byteLength) {
      throw new Error(`Failed encrypting chunk ${chunk} / ${file.chunks} of ${file.metadata?.name}`)
    }

    // console.log(
    //   'Worker',
    //   `Uploading chunk (${encrypted.length} B) ${chunk} / ${file.chunks} of ${file.metadata?.name} - upload attempt ${attempt} (checksum: ${checksum})`
    // )

    const query = {
      chunk,
      checksum
    }

    const headers = {
      'Content-Type': 'application/octet-stream'
    }

    const response = await api.make<Uint8Array, AppFile>(
      'post',
      `/api/storage/${file.id}`,
      query,
      encrypted,
      headers
    )

    // This throw will be caught by the catch block below and handled
    // like all the rest of the possible error responses
    if (!response?.body) {
      throw new Error(
        `Failed uploading chunk ${chunk} / ${file.chunks} of ${file.metadata?.name}, no response body after upload`
      )
    }

    file = {
      ...file,
      ...response.body
    }

    // console.log(
    //   'Worker',
    //   `Done uploading chunk (${encrypted.length} B) ${chunk} / ${file.chunks} of ${file.metadata?.name}`
    // )
  } catch (err) {
    const error = err as ErrorResponse<Uint8Array>

    // If we get checksum error, most likely the data was corrupted during transfer
    // we wont retry indefinitely, but we will try a few times
    if (error.validation?.checksum && attempt < MAX_UPLOAD_RETRIES) {
      console.warn(
        'Worker',
        `Failed uploading chunk ${chunk} / ${file.chunks} of ${file.metadata?.name}, failed checksum, retrying...`
      )
      return uploadChunk(api, file, data, chunk, attempt + 1)
    }

    // The chunk was already uploaded, so we can just return the file
    if (error.validation?.chunk === 'chunk_already_exists') {
      console.warn(
        'Worker',
        `Failed uploading chunk ${chunk} / ${file.chunks} of ${file.metadata?.name}, chunk already exist, skipping...`
      )
    } else {
      console.error(
        'Worker',
        `Failed uploading chunk ${chunk} / ${file.chunks} of ${file.metadata?.name}, either some unexpected error, or too many failed checksum tries, aborting.`,
        err
      )

      throw error
    }
  }

  const transferableFile = {
    ...file,
    started_upload_at: file.started_upload_at || utcStringFromLocal(),
    last_progress_at: utcStringFromLocal(),
    metadata: undefined
  }

  return {
    transferableFile,
    metadataJson: file.metadata?.toJson() || null,
    chunk,
    attempt
  }
}
