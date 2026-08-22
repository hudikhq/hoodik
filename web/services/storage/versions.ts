/**
 * Version-history endpoints for editable files. Mirrors the server's
 * `/api/storage/{file_id}/versions/...` surface added in the
 * versioned-chunks atomicity work.
 *
 * All operations are owner-only. Restore and fork require a transfer
 * token-less call because the heavy chunk copy happens server-side.
 */

import Api from '../api'
import { evictChunkUrls, markDirectTransportBroken, versionChunkUrls } from './download/direct'
import type { AppFile, FileVersion } from 'types'

/**
 * Body the server's `fork` route expects. Mirrors `CreateFile` on the
 * server (every field optional) but we narrow to the ones the client
 * actually sets. `chunks`/`size`/`sha256` are ignored — the server
 * uses the source version's recorded values.
 */
export interface ForkRequest {
  name_hash: string
  encrypted_name: string
  encrypted_thumbnail?: string
  encrypted_key: string
  mime: string
  cipher: string
  editable?: boolean
  file_id?: string
  search_tokens_root?: string[]

  search_tokens_file?: string[]
}

/**
 * `GET /api/storage/{fileId}/versions` — newest-first list of historical
 * snapshots. The active version is intentionally absent (it lives on
 * the file row).
 */
export async function list(fileId: string): Promise<FileVersion[]> {
  const response = await Api.get<FileVersion[]>(`/api/storage/${fileId}/versions`)
  return response?.body || []
}

/**
 * Fetch a single encrypted chunk of a historical version. The caller decrypts
 * with the file's key.
 *
 * Goes straight to the storage bucket when the deployment serves presigned
 * URLs, and through `GET /api/storage/{fileId}/versions/{version}?chunk=N`
 * otherwise. Same manifest module every other read path uses, so a deployment
 * that transfers directly does so here too.
 */
export async function downloadChunk(
  fileId: string,
  version: number,
  chunk: number,
  signal?: AbortSignal
): Promise<Uint8Array> {
  const direct = (await versionChunkUrls(fileId, version))?.[chunk]

  // Credentials would oblige the bucket to answer with an exact-origin CORS
  // policy it does not carry, and are meaningless against a presigned URL.
  let response: Response | undefined
  try {
    response = direct ? await fetch(direct, { credentials: 'omit', signal }) : undefined
  } catch {
    // A broken CORS policy or an unreachable bucket throws instead of
    // answering; heal it the same way an error answer heals below, and stand
    // direct reads down for a while — no fresh manifest fixes a transport
    // that cannot be reached. Aborts rethrow from the relaying read, which
    // shares the signal.
    markDirectTransportBroken()
    response = undefined
  }

  if (direct && (!response || !response.ok)) {
    // An expired URL or a pruned object answers with an XML error document —
    // a body, but not the chunk. Read as ciphertext it would fail much later
    // as a baffling decrypt error, so drop the manifest and take the
    // relaying route for this read instead.
    evictChunkUrls(fileId)
    response = undefined
  }

  if (!response) {
    response = await new Api().download(
      `/api/storage/${fileId}/versions/${version}?chunk=${chunk}`,
      undefined,
      undefined,
      signal
    )
  }

  if (!response.body) {
    throw new Error(`Failed to download chunk ${chunk} of v${version}`)
  }

  const reader = response.body.getReader()
  let data = new Uint8Array(0)
  let done = false

  while (!done) {
    const chunkRead = await reader.read()
    if (chunkRead.value) {
      const merged = new Uint8Array(data.length + chunkRead.value.length)
      merged.set(data, 0)
      merged.set(chunkRead.value, data.length)
      data = merged
    }
    done = chunkRead.done
  }

  return data
}

/**
 * `POST /api/storage/{fileId}/versions/{version}/restore` — pointer
 * flip plus chunk copy server-side. Returns the file with the new
 * `active_version` slot already swapped in.
 */
export async function restore(fileId: string, version: number): Promise<AppFile> {
  const response = await Api.post<undefined, AppFile>(
    `/api/storage/${fileId}/versions/${version}/restore`
  )

  if (!response?.body?.id) {
    throw new Error(`Failed to restore v${version}`)
  }

  // The active version just moved, so any manifest held for this file points
  // at the version it moved away from.
  evictChunkUrls(fileId)

  return response.body
}

/**
 * `POST /api/storage/{fileId}/versions/{version}/fork` —
 * restore-as-new-note. The body is a `CreateFile` (same shape as
 * regular file creation): client builds the encrypted name/key/etc;
 * server copies the source's chunks into the new file's v1 and
 * overrides chunks/size/sha256 with the source version's recorded
 * values.
 */
export async function fork(
  fileId: string,
  version: number,
  newFile: ForkRequest
): Promise<AppFile> {
  const response = await Api.post<ForkRequest, AppFile>(
    `/api/storage/${fileId}/versions/${version}/fork`,
    undefined,
    newFile
  )

  if (!response?.body?.id) {
    throw new Error(`Failed to fork v${version}`)
  }

  return response.body
}

/**
 * `DELETE /api/storage/{fileId}/versions/{version}` — drop a single
 * historical snapshot. The active version cannot be deleted this way.
 */
export async function remove(fileId: string, version: number): Promise<void> {
  await Api.delete(`/api/storage/${fileId}/versions/${version}`)
}

/**
 * `DELETE /api/storage/{fileId}/versions` — wipe every historical
 * snapshot, keeping only the current active version.
 */
export async function purgeAll(fileId: string): Promise<void> {
  await Api.delete(`/api/storage/${fileId}/versions`)
}
