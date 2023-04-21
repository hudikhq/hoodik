import type { KeyPair } from '../../cryptfns/rsa'
import type { ListAppFile } from '../'
import { meta } from '../'
import * as streamer from './streamer'
import * as http from './http'

/**
 * Download the file and decrypt it chunked
 */
export async function chunked(file: ListAppFile): Promise<void> {
  if (!file.metadata?.key) {
    throw new Error("File doesn't have a key, cannot decrypt the data, file is unrecoverable")
  }

  return streamer.download(file)
}

/**
 * Get the file and the files content decrypt the file and its content
 */
export async function get(file: ListAppFile | number, kp: KeyPair): Promise<ListAppFile> {
  if (typeof file === 'number') {
    file = await meta.get(kp, file)
  }

  if (!file.metadata) {
    file.metadata = await meta.FileMetadata.decrypt(file.encrypted_metadata, kp)
  }

  if (!file.metadata.key) {
    throw new Error("File doesn't have a key, cannot decrypt the data, file is unrecoverable")
  }

  file.data = await http.downloadAndDecrypt(file)

  return file
}
