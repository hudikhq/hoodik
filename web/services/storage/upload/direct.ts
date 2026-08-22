import Api from '../../api'
import { transferEncryptedChunkSizes } from 'transfer'
import { orderByChunk, type ChunkUrls } from '../download/direct'
// Straight from the module that defines it, for the same reason the read side
// does: the `!/shares` barrel imports the storage store, and importing it from
// here would close a storage → shares → storage cycle.
import { capabilitiesStore } from '!/shares/capabilities'

import type { AppFile, UploadAppFile } from '../../../types'

/**
 * A file's in-progress direct upload: the URLs it was given, which of its
 * chunks are in the bucket, and how long the URLs stay valid.
 *
 * Unlike a read manifest this one must not outlive the upload it belongs to.
 * The URLs are signed for the version being written, so a note saved twice
 * gets two manifests pointing at two different versions — reusing the first
 * would write the second save's bytes into the first save's slot.
 *
 * `written` starts from the chunks the server already lists as stored, not
 * from empty: a resumed upload writes only the missing ones, and the commit
 * below fires when the bucket holds everything, however many sessions that
 * took.
 */
interface PendingUpload {
  urls: string[]
  written: Set<number>
  expiresAt: number
}

const pending = new Map<string, PendingUpload>()

/**
 * Dropped a minute early so a chunk never PUTs against URLs that expire while
 * it uploads. Same margin the read-side cache keeps.
 */
const EXPIRY_MARGIN_SECONDS = 60

/**
 * Abandon a file's manifest so the next chunk asks for a fresh one. Called on
 * failure, and after the upload is committed.
 */
export function forgetUpload(fileId: string): void {
  pending.delete(fileId)
}

/**
 * Abandon every manifest. Call on logout: the URLs outlive the session that
 * obtained them, and an upload interrupted by a sign-out has no business
 * resuming into the next account's session.
 */
export function forgetAllUploads(): void {
  pending.clear()
}

/**
 * The shape of the content this upload is writing.
 *
 * A file's `size` and `chunks` describe its *active* version. While an edit is
 * in flight they are the previous content's numbers — `replace_content` parks
 * the new ones in `pending_size` and `pending_chunks` and leaves the active
 * version untouched so readers keep seeing it. Declaring `size` mid-edit signs
 * every URL for the wrong content length, and the bucket rejects the body.
 *
 * Same rule the server's own `target_version` follows.
 */
function target(file: UploadAppFile): { size: number; chunks: number } {
  return {
    size: file.pending_size ?? file.size ?? file.file?.size ?? 0,
    chunks: file.pending_chunks ?? file.chunks ?? 0
  }
}

/**
 * Presigned URLs for writing this file's chunks straight into the storage
 * bucket, or `undefined` when this deployment cannot serve them.
 *
 * Fetched once per upload and held until it commits or expires, so the
 * per-chunk callers cost one request for the file rather than one per chunk.
 *
 * Only the chunks the server does not already hold are requested: chunks are
 * write-once, and the server refuses to sign a URL for a stored one, so a
 * fresh request can never target committed ciphertext. Sizes are declared up
 * front because the server signs each one into
 * its URL and the bucket refuses a body of any other length. They come from
 * the crate that does the encrypting rather than from arithmetic here, so the
 * two cannot disagree about the AEAD overhead.
 *
 * `undefined` is a normal answer, not an error: local-filesystem servers have
 * no URLs to give, an S3 server whose bucket failed its startup checks
 * withholds them, and a failure is treated the same way. Every caller uploads
 * through the server instead, which is the path that has always worked. An
 * empty array is different — direct transfer is on and nothing is missing —
 * and keeps the caller on the direct path for the finalize it may still owe.
 *
 * @param api  An `Api` carrying the file's upload transfer token.
 */
export async function uploadChunkUrls(
  file: UploadAppFile,
  api: Api
): Promise<string[] | undefined> {
  // Awaited for the same reason the read side awaits: a gate that reads an
  // unfetched store fails closed and quietly relays.
  const capabilities = capabilitiesStore()
  await capabilities.ensureFetched()
  if (!capabilities.directTransfer) {
    return undefined
  }

  const held = pending.get(file.id)
  if (held) {
    if (held.expiresAt - EXPIRY_MARGIN_SECONDS > Math.floor(Date.now() / 1000)) {
      return held.urls
    }
    pending.delete(file.id)
  }

  try {
    const stored = new Set(file.uploaded_chunks ?? [])
    const sizes = transferEncryptedChunkSizes(file.cipher, target(file).size)

    const chunks = Array.from(sizes, (size, chunk) => ({ chunk, size })).filter(
      ({ chunk }) => !stored.has(chunk)
    )

    // Every chunk already stored is not the relay's case — it is a resume
    // whose predecessor died between its last PUT and finalize. An empty
    // manifest keeps the crate on the direct path, where it uploads nothing
    // and delivers the finalize still owed; `undefined` would send it down
    // the relay arm, which never finalizes, and the file would sit fully
    // stored but uncommitted forever.
    if (!chunks.length) return []

    const response = await api.make<{ chunks: { chunk: number; size: number }[] }, ChunkUrls>(
      'post',
      `/api/storage/${file.id}/upload-urls`,
      undefined,
      { chunks }
    )

    const body = response?.body
    if (!body?.urls?.length) return undefined

    const urls = orderByChunk(body.urls)
    pending.set(file.id, { urls, written: new Set(stored), expiresAt: body.expires_at })

    return urls
  } catch {
    return undefined
  }
}

/**
 * Write one already-encrypted chunk into the bucket, and commit the file once
 * this was the last one outstanding.
 *
 * Returns the committed file, or `undefined` while chunks remain. Nothing
 * tells the server that a bucket write landed, so the client says so — and the
 * server lists the bucket to confirm it before the version pointer moves.
 */
export async function putChunk(
  file: UploadAppFile,
  chunk: number,
  encrypted: Uint8Array,
  url: string,
  api: Api
): Promise<AppFile | undefined> {
  try {
    // No credentials and no custom headers: the presigned URL is signed over
    // the method, the key and the exact content length, so anything else is at
    // best ignored, and at worst turns a simple request into a preflight or an
    // outright rejection.
    const response = await fetch(url, {
      method: 'PUT',
      body: encrypted as BufferSource,
      credentials: 'omit'
    })

    if (!response.ok) {
      throw new Error(`Bucket refused chunk ${chunk} of ${file.id}: ${response.status}`)
    }
  } catch (err) {
    forgetUpload(file.id)
    throw err
  }

  const held = pending.get(file.id)
  if (!held) return undefined

  held.written.add(chunk)
  if (held.written.size < target(file).chunks) return undefined

  forgetUpload(file.id)

  return finalizeUpload(file, api)
}

/**
 * `POST /finalize` — ask the server to list the bucket and commit the target
 * version.
 *
 * Also the answer for an upload with nothing left to PUT: a resume that finds
 * every chunk already stored still has to deliver the finalize its
 * predecessor never got to, or the bytes sit in the bucket forever with the
 * version pointer never moving. The server treats a repeated finalize as a
 * no-op, so callers say it whenever the direct path may have been involved.
 */
export async function finalizeUpload(
  file: UploadAppFile,
  api: Api
): Promise<AppFile | undefined> {
  // The one request that commits a fully-stored upload: a transient failure
  // here must not mark gigabytes of landed ciphertext as failed. Finalize is
  // idempotent, so retrying an ambiguous outcome is safe.
  let lastError: unknown
  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      const committed = await api.make<undefined, AppFile>(
        'post',
        `/api/storage/${file.id}/finalize`
      )

      return committed?.body
    } catch (err) {
      lastError = err
      await new Promise((resolve) => setTimeout(resolve, 500 * (attempt + 1)))
    }
  }

  throw lastError
}
