import type Api from '../../api'
import * as cryptfns from '../../cryptfns'
import * as logger from '!/logger'

import type { DownloadProgressFunction, ListAppFile } from 'types'

/**
 * Create readable stream from downloading chunks and stream them
 * to download of the browser
 */
export async function downloadAndDecryptStream(
  api: Api,
  file: ListAppFile,
  progress: DownloadProgressFunction
): Promise<Response> {
  if (!file.size) {
    throw new Error('Cannot download file without known size')
  }

  const chunkSize = Math.floor(file.size / file.chunks)

  const response = await getResponse(api, file)

  if (!response.body) {
    throw new Error(`Couldn't download file ${file.id}`)
  }

  const reader = response.body.getReader()
  let temp = new Uint8Array(0)
  let decrypted = new Uint8Array(0)
  let downloaded = false

  // @eslint-ignore-next-line
  while (!downloaded) {
    // @ts-ignore
    if ('canceled' in self && self.canceled?.download?.includes(file.id)) {
      throw new Error('Download cancelled')
    }

    const { done, value } = await reader.read()

    logger.debug(`Value size: ${value?.length} ${chunkSize}`)

    if (value) {
      const tg4 = new Uint8Array(temp.length + value.length)
      tg4.set(temp, 0)
      tg4.set(value, temp.length)
      temp = tg4
    }

    if (temp.byteLength === chunkSize) {
      const d = await cryptfns.aes.decrypt(temp, file.metadata?.key as Uint8Array)
      const tg4D = new Uint8Array(decrypted.length + d.length)
      tg4D.set(decrypted, 0)
      tg4D.set(d, decrypted.length)
      decrypted = tg4D
    }

    if (!value && !done) {
      throw new Error("Couldn't download file")
    }

    if (done) {
      downloaded = true
    }

    progress(file, value ? value.length : 0)
  }

  if (temp.byteLength !== 0) {
    const d = await cryptfns.aes.decrypt(temp, file.metadata?.key as Uint8Array)
    const tg4D = new Uint8Array(decrypted.length + d.length)
    tg4D.set(decrypted, 0)
    tg4D.set(d, decrypted.length)
    decrypted = tg4D
  }

  return new Response(new Blob([decrypted]))
}

/**
 * Get the file download response
 */
async function getResponse(api: Api, file: ListAppFile | number): Promise<Response> {
  const id = typeof file === 'number' ? file : file.id

  return await api.download(`/api/storage/${id}`)
}
