import Api from '../../api'
// Straight from the module that defines it, not the `!/shares` barrel: that
// barrel imports the storage store, and importing it from here would close a
// storage → shares → storage cycle.
import { capabilitiesStore } from '!/shares/capabilities'

/**
 * One chunk's presigned URL, as returned by the manifest routes.
 */
interface ChunkUrl {
  chunk: number
  url: string
}

export interface ChunkUrls {
  urls: ChunkUrl[]
  expires_at: number
}

/**
 * A manifest as the transfer crate wants it: indexed by chunk number rather
 * than in whatever order the server listed them.
 *
 * A gap is filled with an empty string rather than left as a hole. The array
 * crosses into wasm as a `Vec<String>`, where a hole is not a string, and the
 * crate reads an empty entry as "this one goes through the server".
 */
export function orderByChunk(urls: ChunkUrl[]): string[] {
  const highest = urls.reduce((max, { chunk }) => Math.max(max, chunk), 0)
  const ordered: string[] = new Array(highest + 1).fill('')
  for (const { chunk, url } of urls) {
    ordered[chunk] = url
  }
  return ordered
}

interface CachedManifest {
  urls: string[]
  expiresAt: number
  fetchedAt: number
}

/**
 * Manifests already fetched this session, keyed by file or link id.
 *
 * Progressive consumers build a downloader per chunk — a video feeding
 * MediaSource asks for one chunk at a time — so without this a manifest would
 * be refetched once per chunk of the file it describes. The URLs stay valid
 * for as long as the server signed them for, which is measured in days, so
 * one fetch covers every chunk of a playback session.
 */
const cache = new Map<string, CachedManifest>()

/**
 * Dropped a minute early so a transfer never starts against URLs that expire
 * while it runs.
 */
const EXPIRY_MARGIN_SECONDS = 60

/**
 * How long a manifest may be reused before the server is asked again.
 *
 * Far shorter than the URLs stay valid, and deliberately so: fetching a
 * manifest is what runs the read gate, and a share revoked elsewhere reaches
 * this client no other way. Signed for days, a manifest would otherwise let a
 * recipient keep reading a file they had lost access to for as long as the
 * tab stayed open. Long enough that a playback session still costs one fetch.
 */
const REUSE_SECONDS = 300

function cached(key: string): string[] | undefined {
  const hit = cache.get(key)
  if (!hit) return undefined

  const now = Math.floor(Date.now() / 1000)
  if (hit.expiresAt - EXPIRY_MARGIN_SECONDS <= now || now - hit.fetchedAt >= REUSE_SECONDS) {
    cache.delete(key)
    return undefined
  }

  return hit.urls
}

/**
 * How long direct reads stand down after a transport-level failure.
 *
 * An error *answer* — an expired URL, a pruned object — heals by eviction,
 * because a fresh manifest fixes it. A thrown fetch is different: broken
 * bucket CORS or an unreachable endpoint, which no fresh manifest fixes,
 * and without a stand-down every read of the session pays one failed bucket
 * request plus a manifest round-trip before it relays.
 */
const TRANSPORT_COOLDOWN_SECONDS = 60

let transportBrokenAt: number | null = null

/**
 * Report that a presigned read failed at the transport level. Manifests stop
 * being handed out for [[TRANSPORT_COOLDOWN_SECONDS]], so reads relay
 * immediately instead of each rediscovering the same broken transport.
 */
export function markDirectTransportBroken(): void {
  transportBrokenAt = Math.floor(Date.now() / 1000)
}

function transportCoolingDown(): boolean {
  if (transportBrokenAt === null) return false
  if (Math.floor(Date.now() / 1000) - transportBrokenAt >= TRANSPORT_COOLDOWN_SECONDS) {
    transportBrokenAt = null
    return false
  }
  return true
}

/**
 * Forget every cached manifest. Call on logout: the URLs outlive the session
 * that obtained them, and nothing should carry across an account switch.
 */
export function clearChunkUrlCache(): void {
  cache.clear()
  transportBrokenAt = null
}

/**
 * Forget one file's manifest, across its active version and every historical
 * one.
 *
 * A manifest describes the chunks of the version that was active when it was
 * signed, and the entries live for as long as the URLs do — days. So after the
 * content is replaced or a version is restored, a cached manifest points at the
 * previous version's chunks: the download either returns the old bytes or, if
 * the pointer moved mid-fetch, a mix of two versions that fails to decrypt.
 */
export function evictChunkUrls(fileId: string): void {
  for (const key of [...cache.keys()]) {
    if (key.startsWith(`file:${fileId}`) || key.startsWith(`version:${fileId}:`)) {
      cache.delete(key)
    }
  }
}

/**
 * Forget one link's manifest. Link manifests are keyed by link id — the page
 * that holds them never learns which file sits behind the link — so an edit
 * to the shared content reaches them only through failure: a fetch or decrypt
 * that fails evicts here and the read retries through the server.
 */
export function evictLinkChunkUrls(linkId: string): void {
  cache.delete(`link:${linkId}`)
}

/**
 * Presigned URLs for every chunk, ordered by chunk index, or `undefined` when
 * this deployment cannot serve them.
 *
 * Returning `undefined` is the normal answer, not an error: local-filesystem
 * servers have no URLs to give, and an S3 server whose bucket failed its
 * startup checks deliberately withholds them. Every caller falls back to
 * fetching through the server, which is the path that has always worked.
 *
 * A failure here is treated the same way. A transfer that would have been
 * faster is not worth a transfer that does not happen.
 */
async function chunkUrls(
  key: string,
  path: string,
  method: 'get' | 'post'
): Promise<string[] | undefined> {
  // Awaited rather than read: the public link page never logs in, so nothing
  // has fetched the advertisement there — a bare read sees the fail-closed
  // null and silently relays every link download on a deployment that serves
  // URLs perfectly well.
  const capabilities = capabilitiesStore()
  await capabilities.ensureFetched()
  if (!capabilities.directTransfer) {
    return undefined
  }

  if (transportCoolingDown()) {
    return undefined
  }

  const hit = cached(key)
  if (hit) return hit

  try {
    const response =
      method === 'get'
        ? await Api.get<ChunkUrls>(path)
        : await Api.post<undefined, ChunkUrls>(path)

    const body = response?.body
    if (!body?.urls?.length) return undefined

    const ordered = orderByChunk(body.urls)

    cache.set(key, {
      urls: ordered,
      expiresAt: body.expires_at,
      fetchedAt: Math.floor(Date.now() / 1000)
    })

    return ordered
  } catch {
    return undefined
  }
}

/**
 * Presigned URLs for an authenticated file's active version.
 *
 * Keyed by that version, not by the file: the URLs address the chunks of the
 * version they were signed for, and they outlive it by days. Cached under the
 * file id alone, a tab that had read a note once kept serving the content it
 * read first — every later edit, from this device or another, invisible to it
 * until the entry expired. The caller reads the version off a row it has just
 * fetched, so a client that learns about an edit misses the stale entry on its
 * own.
 */
export async function fileChunkUrls(
  fileId: string,
  activeVersion: number
): Promise<string[] | undefined> {
  return chunkUrls(`file:${fileId}:${activeVersion}`, `/api/storage/${fileId}/chunk-urls`, 'get')
}

/**
 * Presigned URLs for a public share link.
 */
export async function linkChunkUrls(linkId: string): Promise<string[] | undefined> {
  return chunkUrls(`link:${linkId}`, `/api/links/${linkId}/chunk-urls`, 'post')
}

/**
 * Presigned URLs for a historical version of a file.
 */
export async function versionChunkUrls(
  fileId: string,
  version: number
): Promise<string[] | undefined> {
  return chunkUrls(
    `version:${fileId}:${version}`,
    `/api/storage/${fileId}/versions/${version}/chunk-urls`,
    'get'
  )
}
