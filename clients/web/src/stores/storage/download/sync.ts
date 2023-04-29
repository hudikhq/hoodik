import * as cryptfns from '../../cryptfns'
import Api from '../../api'

import type { DownloadProgressFunction, ListAppFile } from '../../../types'

/**
 * Download the file content
 */
export async function downloadAndDecrypt(file: ListAppFile): Promise<Uint8Array> {
  let data = new Uint8Array(0)

  for (let i = 0; i < file.chunks; i++) {
    const encrypted = await downloadEncryptedChunk(file, i)
    const chunk = cryptfns.aes.decrypt(encrypted, file?.metadata?.key as Uint8Array)
    const tg4 = new Uint8Array(data.length + chunk.length)
    tg4.set(data, 0)
    tg4.set(chunk, data.length)
    data = tg4
  }

  return data
}

/**
 * Create readable stream from downloading chunks and stream them
 * to download of the browser
 */
export async function downloadAndDecryptStream(
  file: ListAppFile,
  progress?: DownloadProgressFunction
) {
  const chunks = [...new Array(file.chunks)].map((_, i) => i)

  const stream = new ReadableStream({
    start: async () => {
      if (progress) {
        await progress(file, 0)
      }
    },
    pull: async (controller) => {
      const chunk = chunks.shift()

      if (!chunk && chunk !== 0) {
        controller.close()
        return
      }

      const data = await downloadChunk(file, chunk as number)
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

  const response = new Response(stream)
  const url = window.URL.createObjectURL(await response.blob())
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = file.metadata?.name as string
  anchor.click()
  window.URL.revokeObjectURL(url)
}

/**
 * Download single file chunk and decrypt it
 */
export async function downloadChunk(file: ListAppFile, chunk: number): Promise<Uint8Array> {
  const data = await downloadEncryptedChunk(file, chunk)
  return cryptfns.aes.decrypt(data, file?.metadata?.key as Uint8Array)
}

/**
 * Download a single chunk of the file and return it without decrypting it
 */
export async function downloadEncryptedChunk(
  file: ListAppFile,
  chunk: number
): Promise<Uint8Array> {
  const response = await getResponse(file, chunk)

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
async function getResponse(file: ListAppFile | number, chunk: number): Promise<Response> {
  const id = typeof file === 'number' ? file : file.id

  return await new Api().download(`/api/storage/${id}?chunk=${chunk}`)
}
