import * as cryptfns from '!/cryptfns'
import * as crypto from './crypto'
import Api from '!/api'

import type { AppLink, CreateLink, EncryptedAppLink, KeyPair, ListAppFile } from 'types'

/**
 * Load all the shared links for the user.
 */
export async function all(): Promise<EncryptedAppLink[]> {
  const response = await Api.get<EncryptedAppLink[]>(`/api/links`)

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
  return new Api().download<{ link_key: string }>(`/api/links/${id}`, undefined, {
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
  const response = await Api.get<EncryptedAppLink>(`/api/links/${id}`)

  if (!response.body) {
    throw new Error('Failed to get link')
  }

  return response.body
}

/**
 * Convert unencrypted app file into a encrypted create link construct
 */
export async function createLinkFromFile(file: ListAppFile, kp: KeyPair): Promise<CreateLink> {
  const key = await cryptfns.aes.generateKey()

  const signature = await cryptfns.rsa.sign(kp, file.id)
  const encrypted_link_key = await cryptfns.rsa.encryptMessage(
    cryptfns.uint8.toHex(key),
    kp.publicKey as string
  )

  const encrypted_name = await cryptfns.aes.encryptToHex(file.metadata?.name || 'no-name', key)
  const encrypted_file_key = await cryptfns.aes.encryptToHex(cryptfns.uint8.toHex(key), key)

  return {
    file_id: file.id,
    signature,
    encrypted_link_key,
    encrypted_name,
    encrypted_file_key
  }
}
