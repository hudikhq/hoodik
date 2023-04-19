import type { KeyPair } from '../cryptfns/rsa'
import type { AppFile } from './meta'
import * as cryptfns from '../cryptfns'
import { meta } from '.'
import Api from '../api'

/**
 * Get the file and the files content decrypt the file and its content
 */
export async function get(file: AppFile | number, kp: KeyPair): Promise<AppFile> {
  if (typeof file === 'number') {
    file = await meta.get(kp, file)
  }

  if (!file.metadata) {
    file.metadata = await meta.FileMetadata.decrypt(file.encrypted_metadata, kp)
  }

  if (!file.metadata.key) {
    throw new Error("File doesn't have a key, cannot decrypt the data, file is unrecoverable")
  }

  file.data = cryptfns.aes.decrypt(await download(file), file.metadata.key)

  return file
}

/**
 * Download the file content
 */
export async function download(file: AppFile): Promise<Uint8Array> {
  const response = await getResponse(file)

  if (!response.body) {
    throw new Error("Couldn't download file")
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
      return data
    }
  }

  throw new Error("Couldn't download file")
}

/**
 * Get the file download response
 */
export async function getResponse(file: AppFile | number): Promise<Response> {
  const id = typeof file === 'number' ? file : file.id

  return await Api.download(`/api/storage/${id}`)
}
