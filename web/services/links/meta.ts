import * as cryptfns from '!/cryptfns'
import * as crypto from './crypto'
import * as storageMeta from '!/storage/meta'
import Api from '!/api'
import { CHUNK_SIZE_BYTES } from '!/constants'

import type { AppLink, CreateLink, EncryptedAppLink, KeyPair, AppFile } from 'types'

/**
 * Load all the shared links for the user.
 */
export async function all(): Promise<EncryptedAppLink[]> {
  const response = await Api.get<EncryptedAppLink[]>(`/api/links`, {
    with_expired: 'true',
    compact: true
  })

  if (!Array.isArray(response.body)) {
    throw new Error('Failed to get link')
  }

  return response.body
}

/**
 * Fetch and decrypt the full link content client-side.
 *
 * The server stores the file as N independently-encrypted chunks; we pull each
 * one raw (`?chunk=i`, no link_key sent) and decrypt it with the file cipher and
 * the content key the caller unwrapped from the link metadata, concatenating the
 * plaintext. This keeps the E2EE guarantee intact: the server never decrypts.
 */
export async function downloadAndDecrypt(link: AppLink): Promise<Uint8Array> {
  const key = link.key
  if (!key) {
    throw new Error('Cannot decrypt link content without the file key')
  }

  const chunks = Math.max(1, Math.ceil((link.file_size || 0) / CHUNK_SIZE_BYTES))
  const api = new Api()

  let data = new Uint8Array(0)

  for (let i = 0; i < chunks; i++) {
    const response = await api.postDownload(`/api/links/${link.id}?chunk=${i}`, undefined, undefined)

    if (!response.ok) {
      throw new Error(`Could not download link ${link.id}, chunk: ${i}`)
    }

    const encrypted = new Uint8Array(await response.arrayBuffer())
    const chunk = await cryptfns.cipher.decrypt(link.file_cipher, encrypted, key, i)

    const next = new Uint8Array(data.length + chunk.length)
    next.set(data, 0)
    next.set(chunk, data.length)
    data = next
  }

  return data
}

/**
 * Decrypt the link content client-side and trigger a browser save under the
 * name from the (client-decrypted) link metadata.
 */
export async function saveDecrypted(link: AppLink): Promise<void> {
  const data = await downloadAndDecrypt(link)

  const url = window.URL.createObjectURL(new Blob([data], { type: link.file_mime }))
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = link.name || 'download'
  anchor.click()
  window.URL.revokeObjectURL(url)
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

  const identity = kp.input as string
  const wrapPub = (kp as any).wrappingPublic || (kp.publicKey as string)

  const signature = crypto.isCurveKey(identity)
    ? await cryptfns.ed25519.sign(file.id, identity)
    : await cryptfns.rsa.sign(kp, file.id)

  const encrypted_link_key = crypto.isCurveKey(wrapPub)
    ? await cryptfns.wrapping.wrap(key, wrapPub)
    : await cryptfns.rsa.encryptMessage(cryptfns.uint8.toHex(key), wrapPub)

  const encrypted_name = await cryptfns.cipher.encryptString(crypto.LINK_CIPHER, file.name || 'no-name', key)
  const encrypted_file_key = await cryptfns.cipher.encryptString(
    crypto.LINK_CIPHER,
    cryptfns.uint8.toHex(file.key),
    key
  )

  // Listings no longer carry thumbnail blobs, so pull it from the
  // thumbnail route when the row only advertises one — the link keeps
  // its own copy encrypted under the link key.
  let thumbnail = file.thumbnail
  if (!thumbnail && file.has_thumbnail) {
    const encrypted = await storageMeta.thumbnail(file.id)
    if (encrypted) {
      thumbnail = await cryptfns.cipher.decryptString(file.cipher, encrypted, file.key)
    }
  }

  let encrypted_thumbnail

  if (thumbnail) {
    encrypted_thumbnail = await cryptfns.cipher.encryptString(crypto.LINK_CIPHER, thumbnail, key)
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
