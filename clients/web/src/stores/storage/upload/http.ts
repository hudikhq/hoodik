import Api, { ErrorResponse, type Query } from '../../api'
import * as cryptfns from '../../cryptfns'
import { CHUNK_SIZE_BYTES, MAX_RETRIES, UPLOAD_BATCH } from '.'
import type { UploadAppFile, ProgressFunction } from '.'
import type { AppFile } from '../meta'

/**
 * Upload single file from the upload queue
 */
export async function upload(file: UploadAppFile, progress?: ProgressFunction) {
  if (!file.started_upload_at) {
    file.started_upload_at = new Date()
  }
  if (progress) {
    await progress(file, false)
  }

  const workers = [...new Array(file.chunks)].map((_, chunk) => {
    return async () => {
      // Skip already uploaded chunks
      if (file.uploaded_chunks?.includes(chunk)) {
        if (progress) {
          await progress(file, chunk === file.chunks - 1)
        }

        return file
      }

      const data = await sliceChunk(file.file, chunk)

      file = await uploadChunk(file, data, chunk)

      console.log(`Uploaded chunk ${chunk} / ${file.chunks} of ${file.file.name}`)

      if (progress) {
        await progress(file, chunk === file.chunks - 1)
      }

      return file
    }
  })

  while (workers.length) {
    const batch = workers.splice(0, UPLOAD_BATCH)
    const files = await Promise.all(batch.map((worker) => worker()))
    file = files[files.length - 1]
  }

  return file
}

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

    const uploaded = await Api.upload<AppFile>(`/api/storage/${file.id}`, encrypted, query, headers)

    return {
      ...uploaded,
      metadata: file.metadata,
      file: file.file,
      started_upload_at: file.started_upload_at || new Date()
    }
  } catch (err) {
    const error = err as ErrorResponse<Uint8Array>

    // If we get checksum error, most likely the data was corrupted during transfer
    // we wont retry indefinitely, but we will try a few times
    if (error.validation?.checksum && attempt < MAX_RETRIES) {
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

/**
 * Perform slicing of the file chunk with a fallback in case
 * the browser does not support the arrayBuffer method on the Blob
 */
async function sliceChunk(file: File, chunk: number): Promise<Uint8Array> {
  const start = chunk * CHUNK_SIZE_BYTES
  const end = (chunk + 1) * CHUNK_SIZE_BYTES

  const slice = file.slice(start, end)

  if (typeof slice.arrayBuffer === 'function') {
    return new Uint8Array(await slice.arrayBuffer())
  }

  return new Promise((resolve, reject) => {
    const reader = new FileReader()

    reader.onload = () => {
      if (reader.result instanceof ArrayBuffer) {
        resolve(new Uint8Array(reader.result))
      } else {
        reject(new Error('Failed to read file'))
      }
    }

    reader.onerror = (err) => {
      reject(err)
    }

    reader.readAsArrayBuffer(slice)
  })
}
