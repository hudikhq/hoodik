import * as cryptfns from '!/cryptfns'
import * as crypto from './crypto'
import * as storageMeta from '!/storage/meta'
import Api from '!/api'
import { CHUNK_SIZE_BYTES } from '!/constants'
import { TransferDownloader } from 'transfer'
import { linkChunkUrls } from '!/storage/download/direct'
import { buildDownloader } from '!/storage/download/downloader'

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
 * Number of encrypted chunks behind a link, derived the same way the
 * uploader sliced them.
 */
export function linkChunks(link: AppLink): number {
  return Math.max(1, Math.ceil((link.file_size || 0) / CHUNK_SIZE_BYTES))
}

/**
 * Build a wasm downloader for a public link. Chunks come from the anonymous
 * link route; the content key was unwrapped from the link metadata with the
 * fragment key and never leaves the browser — the server only ever streams
 * ciphertext. Callers must `free()` the downloader.
 */
async function linkDownloader(link: AppLink): Promise<TransferDownloader> {
  if (!link.key) {
    throw new Error('Cannot decrypt link content without the file key')
  }

  // Built by the same factory as every authenticated read, so the link route
  // cannot quietly miss what the rest of the client gained. It stays on this
  // thread: a share-link page is not signed in and never starts the workers.
  return buildDownloader(
    {
      id: link.id,
      size: link.file_size || 0,
      chunks: linkChunks(link),
      cipher: link.file_cipher,
      key: link.key as Uint8Array,
      directUrls: await linkChunkUrls(link.id),
      publicLink: true
    },
    new Api().toJson()
  )
}

/**
 * Fetch and decrypt the full link content client-side through the wasm
 * transfer pipeline — concurrent chunk downloads, decryption and ordering
 * all happen inside the crate.
 */
export async function downloadAndDecrypt(
  link: AppLink,
  onBytes?: (bytes: number) => void
): Promise<Uint8Array> {
  const downloader = await linkDownloader(link)

  try {
    return await downloader.download((progressJson: string) => {
      if (!onBytes) return

      const progress = JSON.parse(progressJson)
      if (progress.type === 'download' && typeof progress.bytes_downloaded === 'number') {
        onBytes(progress.bytes_downloaded)
      }
    }, () => false)
  } finally {
    downloader.free()
  }
}

/**
 * Download and decrypt a single chunk of a public link — random access for
 * progressive video playback.
 */
export async function downloadLinkChunk(
  link: AppLink,
  chunk: number,
  signal?: AbortSignal
): Promise<Uint8Array> {
  if (signal?.aborted) {
    throw new DOMException('Download aborted', 'AbortError')
  }

  const downloader = await linkDownloader(link)

  try {
    return await downloader.downloadChunk(chunk, undefined)
  } finally {
    downloader.free()
  }
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
