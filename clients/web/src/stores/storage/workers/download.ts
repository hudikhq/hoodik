import Api from '@/stores/api'
import { DOWNLOAD_POOL_LIMIT } from '../constants'
import * as cryptfns from '@/stores/cryptfns'

import type { ListAppFile } from '../types'

export type DownloadIteratorFunction = (
  file: ListAppFile,
  i: number,
  items: number[]
) => Promise<Uint8Array>

/**
 * Start downloading concurrently chunks of the file
 */
export async function download(api: Api, file: ListAppFile): Promise<void> {
  const buffer = await start(api, file)
  return saveAs(file, buffer)
}

/**
 * Initiate the browser download action
 */
function saveAs(file: ListAppFile, buffers: Uint8Array): void {
  const blob = new Blob([buffers], { type: file.mime })
  const blobUrl = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.download = file.metadata?.name || file.id.toString()
  a.href = blobUrl
  a.click()
  URL.revokeObjectURL(a.href)
  console.log('File downloaded')
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
  file: ListAppFile,
  iterable: number[],
  iteratorFn: DownloadIteratorFunction
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
 * Start by creating a pool of asynchronous tasks that will download
 * chunks and decrypt them.
 */
async function start(api: Api, file: ListAppFile): Promise<Uint8Array> {
  if (!file.size || !file.chunks || !file.finished_upload_at || !file.metadata) {
    throw new Error(`File ${file.id} is not available for download`)
  }

  const results = await asyncPool(
    DOWNLOAD_POOL_LIMIT,
    file,
    [...new Array(file.chunks).keys()],
    async (file: ListAppFile, i: number) => {
      const content = await downloadChunk(api, file, i)

      const transferableFile = { ...file, metadata: undefined }

      self.postMessage({
        type: 'download-progress',
        message: {
          transferableFile,
          metadataJson: file.metadata?.toJson() || null,
          chunkBytes: content.byteLength
        }
      })

      return content
    }
  )

  const sortedBuffers = results.map((item) => new Uint8Array(item.buffer))

  return concatenate(sortedBuffers)
}

/**
 * Download single file chunk and decrypt it
 */
export async function downloadChunk(
  api: Api,
  file: ListAppFile,
  chunk: number
): Promise<Uint8Array> {
  const data = await downloadEncryptedChunk(api, file, chunk)
  return cryptfns.aes.decrypt(data, file?.metadata?.key as Uint8Array)
}

/**
 * Download a single chunk of the file and return it without decrypting it
 */
export async function downloadEncryptedChunk(
  api: Api,
  file: ListAppFile,
  chunk: number
): Promise<Uint8Array> {
  const response = await getResponse(api, file, chunk)

  if (!response.body) {
    throw new Error(`Couldn't download file ${file.id}, chunk: ${chunk}`)
  }

  const reader = response.body.getReader()
  let data = new Uint8Array(0)
  let downloaded = false

  // @eslint-ignore-next-line
  while (!downloaded) {
    const { done, value } = await reader.read()

    if (value) {
      const tg4 = new Uint8Array(data.length + value.length)
      tg4.set(data, 0)
      tg4.set(value, data.length)
      data = tg4
    }

    if (!value && !done) {
      throw new Error("Couldn't download file")
    }

    if (done) {
      downloaded = true
      const checksum = cryptfns.sha256.digest(data)
      console.log(`Downloaded chunk (${data.length} B) ${chunk} of ${file.chunks} - ${checksum}`)
      return data
    }
  }

  throw new Error(`Couldn't download file ${file.id}, chunk: ${chunk}`)
}

/**
 * Get the file download response
 */
async function getResponse(api: Api, file: ListAppFile | number, chunk: number): Promise<Response> {
  const id = typeof file === 'number' ? file : file.id

  const { request, fetchOptions } = Api.buildRequest(
    'get',
    `/api/storage/${id}?chunk=${chunk}`,
    undefined,
    undefined,
    undefined,
    api
  )

  return fetch(decodeURIComponent(request.url), fetchOptions)
}
