import type Api from '@/stores/api'
import * as cryptfns from '@/stores/cryptfns'

import type { DownloadProgressFunction, ListAppFile } from '../../../types'

/**
 * Create readable stream from downloading chunks and stream them
 * to download of the browser
 */
export async function downloadAndDecryptStream(
  api: Api,
  file: ListAppFile,
  progress: DownloadProgressFunction
): Promise<Response> {
  const chunks = [...new Array(file.chunks)].map((_, i) => i)

  const stream = new ReadableStream<Uint8Array>({
    start: async () => {
      if (progress) {
        await progress(file, 0)
      }
    },
    pull: async (controller) => {
      const chunk = chunks.shift()

      // @ts-ignore
      if ('canceled' in self && self.canceled?.download?.includes(file.id)) {
        throw new Error('Download cancelled')
      }

      if (!chunk && chunk !== 0) {
        controller.close()
        return
      }

      const data = await downloadChunk(api, file, chunk as number)
      if (data) {
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
      // const checksum = cryptfns.sha256.digest(data)
      // console.log(`Downloaded chunk (${data.length} B) ${chunk} of ${file.chunks} - ${checksum}`)
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
