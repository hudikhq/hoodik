import type Api from '../../api'
import * as cryptfns from '../../cryptfns'
// import * as logger from '!/logger'

import type { DownloadProgressFunction, ListAppFile } from 'types'
import { DOWNLOAD_POOL_LIMIT } from '../../constants'

/**
 * Create readable stream from downloading chunks and stream them
 * to download of the browser
 */
export async function downloadAndDecryptStream(
  api: Api,
  file: ListAppFile,
  progress: DownloadProgressFunction
): Promise<Response> {
  const fileChunks = [...new Array(file.chunks)].map((_, i) => i)

  const stream = new ReadableStream<Uint8Array>({
    start: async () => {
      if (progress) {
        await progress(file, 0)
      }
    },
    pull: async (controller) => {
      const chunks = []

      if (fileChunks.length > 0) {
        for (let i = 0; i < DOWNLOAD_POOL_LIMIT; i++) {
          const chunk = fileChunks.shift()

          if (typeof chunk === 'number') {
            chunks.push(chunk)
          }
        }
      }

      // @ts-ignore
      if ('canceled' in self && self.canceled?.download?.includes(file.id)) {
        throw new Error('Download cancelled')
      }

      if (chunks.length === 0) {
        controller.close()
        return
      }

      let data = new Uint8Array(0)

      const results = await Promise.all(
        chunks.map((c) => downloadChunk(self.SWApi || api, file, c as number))
      )

      for (const result of results) {
        const tg4 = new Uint8Array(data.length + result.length)
        tg4.set(data, 0)
        tg4.set(result, data.length)
        data = tg4
      }

      console.log(data.length / 1024 / 1024)
      if (data.length) {
        if (progress) {
          await progress(file, data.length)
        }

        return controller.enqueue(data)
      } else {
        controller.close()
      }
    }
  })

  return new Response(stream)
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
  return cryptfns.worker.decrypt(data, file?.metadata?.key as Uint8Array)
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
      // const checksum = cryptfns.sha256.digest(data)
      // logger.debug(`Downloaded chunk (${data.length} B) ${chunk} of ${file.chunks} - ${checksum}`)
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

  return await api.download(`/api/storage/${id}?chunk=${chunk}`)
}
