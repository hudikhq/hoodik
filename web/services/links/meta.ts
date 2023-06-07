import * as cryptfns from '!/cryptfns'
import * as crypto from './crypto'
import Api from '!/api'

import type { AppLink, CreateLink, EncryptedAppLink, KeyPair, AppFile } from 'types'

/**
 * Load all the shared links for the user.
 */
export async function all(): Promise<EncryptedAppLink[]> {
  const response = await Api.get<EncryptedAppLink[]>(`/api/links`, {
    with_expired: 'true'
  })

  if (!Array.isArray(response.body)) {
    throw new Error('Failed to get link')
  }

  return response.body
}

/**
 * Download the link by its id, the download on the server will decrypt file and
 * return the file as a response.
 */
export async function download(id: string, link_key: string): Promise<Response> {
  return new Api().postDownload<{ link_key: string }>(`/api/links/${id}`, undefined, {
    link_key
  })
}

/**
 * Run the download by mocking a form submit
 */
export async function formDownload(id: string, link_key: string): Promise<void> {
  return new Api().formDownload<{ link_key: string }>(`/api/links/${id}`, undefined, {
    link_key
  })
}

/**
 * Load the link by its id and its metadata from the server.
 */
export async function metadata(id: string, linkKey: string): Promise<AppLink> {
  const link = await encryptedMetadata(id)

  return crypto.decryptLink(link, linkKey)
}

/**
 * Get the encrypted metadata in case we don't have a key
 */
export async function encryptedMetadata(id: string): Promise<EncryptedAppLink> {
  const response = await Api.get<EncryptedAppLink>(`/api/links/${id}/metadata`)

  if (!response.body) {
    throw new Error('Failed to get link')
  }

  return response.body
}

/**
 * Convert unencrypted app file into a encrypted create link construct
 */
export async function createLinkFromFile(file: AppFile, kp: KeyPair): Promise<CreateLink> {
  if (!file.key) {
    throw new Error('File key is missing')
  }

  const key = await cryptfns.aes.generateKey()

  const signature = await cryptfns.rsa.sign(kp, file.id)
  const encrypted_link_key = await cryptfns.rsa.encryptMessage(
    cryptfns.uint8.toHex(key),
    kp.publicKey as string
  )

  const encrypted_name = await cryptfns.aes.encryptToHex(file.name || 'no-name', key)
  const encrypted_file_key = await cryptfns.aes.encryptToHex(cryptfns.uint8.toHex(file.key), key)

  let encrypted_thumbnail

  if (file.thumbnail) {
    encrypted_thumbnail = await cryptfns.aes.encryptToHex(file.thumbnail, key)
  }

  return {
    file_id: file.id,
    signature,
    encrypted_link_key,
    encrypted_name,
    encrypted_file_key,
    encrypted_thumbnail
  }
}
