import type { UploadAppFile } from '.'
import * as http from './http'

export const POOL_LIMIT = 10
export type IteratorFunction = (
  file: UploadAppFile,
  i: number,
  items: number[]
) => Promise<Uint8Array>
export type ProgressFunction = (
  file: UploadAppFile,
  chunk: number,
  byteLength?: number
) => Promise<void>

/**
 * Start downloading concurrently chunks of the file
 */
export async function upload(file: UploadAppFile, progress?: ProgressFunction): Promise<void> {
  const buffer = await start(file, progress)
  return saveAs(file, buffer)
}

/**
 * Initiate the browser download action
 */
function saveAs(file: UploadAppFile, buffers: Uint8Array): void {
  const blob = new Blob([buffers], { type: file.mime })
  const blobUrl = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.download = file.metadata?.name || file.id.toString()
  a.href = blobUrl
  a.click()
  URL.revokeObjectURL(a.href)
}

/**
 * Concatenate multiple uint8 arrays together
 */
function concatenate(arrays: Uint8Array[]): Uint8Array {
  if (!arrays.length) return new Uint8Array(0)

  const totalLength = arrays.reduce((acc, value) => acc + value.length, 0)
  const result = new Uint8Array(totalLength)
  let length = 0
  for (const array of arrays) {
    result.set(array, length)
    length += array.length
  }
  return result
}

/**
 * Create a pool of asynchronous tasks that will download chunks and decrypt them.
 */
async function asyncPool(
  concurrency: number,
  file: UploadAppFile,
  iterable: number[],
  iteratorFn: IteratorFunction
): Promise<Uint8Array[]> {
  const ret = [] // Store all asynchronous tasks
  const executing = new Set() // Stores executing asynchronous tasks
  for (const item of iterable) {
    // Call the iteratorFn function to create an asynchronous task
    const p = Promise.resolve().then(() => iteratorFn(file, item, iterable))

    ret.push(p) // save new async task
    executing.add(p) // Save an executing asynchronous task

    const clean = () => executing.delete(p)
    p.then(clean).catch(clean)
    if (executing.size >= concurrency) {
      // Wait for faster task execution to complete
      await Promise.race(executing)
    }
  }
  return Promise.all(ret)
}

/**
 * Start by creating a pool of asynchronous tasks that will upload
 * chunks and decrypt them.
 */
async function start(file: UploadAppFile, progress?: ProgressFunction): Promise<Uint8Array> {
  if (!file.size || !file.chunks || !file.finished_upload_at || !file.metadata) {
    throw new Error(`File ${file.id} is not available for upload`)
  }

  const results = await asyncPool(
    POOL_LIMIT,
    file,
    [...new Array(file.chunks).keys()],
    async (file: UploadAppFile, i: number) => {
      const content = await http.uploadChunk(file, i)

      if (progress) {
        await progress(file, i, content.byteLength)
      }

      return content
    }
  )

  const sortedBuffers = results.map((item) => new Uint8Array(item.buffer))

  return concatenate(sortedBuffers)
}
