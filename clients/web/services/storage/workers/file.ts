import { utcStringFromLocal } from '../../'
import { uploadChunk } from './chunk'
import type { UploadAppFile } from 'types'
import type { ErrorResponse } from '../../api'
import type Api from '../../api'
import { CHUNK_SIZE_BYTES, CONCURRENT_CHUNKS_UPLOAD } from '../constants'

/**
 * Run chunked file upload from start to end in the worker
 */
export async function uploadFile(
  api: Api,
  file: UploadAppFile,
  progress: (
    file: UploadAppFile,
    attempt: number,
    isDone: boolean,
    error?: Error | ErrorResponse<any> | string | undefined
  ) => void
) {
  if (!file.started_upload_at) {
    file.started_upload_at = utcStringFromLocal()
  }

  progress(file, 0, false)

  const workers = [...new Array(file.chunks)].map((_, chunk) => {
    return async () => {
      if ('canceled' in self && self.canceled?.upload?.includes(file.id)) {
        throw new Error('Upload cancelled')
      }

      // Skip already uploaded chunks
      if (file.uploaded_chunks?.includes(chunk)) {
        return
      }

      const data = await sliceChunk(file.file as File, chunk)

      const response = await uploadChunk(api, file, data, chunk, 0)

      const storedChunks = response.transferableFile.uploaded_chunks?.length || 0

      progress(response.transferableFile, response.attempt, storedChunks === file.chunks)
    }
  })

  while (workers.length) {
    const batch = workers.splice(0, CONCURRENT_CHUNKS_UPLOAD)
    await Promise.race(batch.map((worker) => worker()))
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
