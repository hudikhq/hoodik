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

interface ChunkUrls {
  urls: ChunkUrl[]
  expires_at: number
}

interface CachedManifest {
  urls: string[]
  expiresAt: number
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

function cached(key: string): string[] | undefined {
  const hit = cache.get(key)
  if (!hit) return undefined

  if (hit.expiresAt - EXPIRY_MARGIN_SECONDS <= Math.floor(Date.now() / 1000)) {
    cache.delete(key)
    return undefined
  }

  return hit.urls
}

/**
 * Forget every cached manifest. Call on logout: the URLs outlive the session
 * that obtained them, and nothing should carry across an account switch.
 */
export function clearChunkUrlCache(): void {
  cache.clear()
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
  if (!capabilitiesStore().directTransfer) {
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

    // Indexed by chunk number rather than trusting response order. A gap
    // leaves that index undefined, and the crate sends those through the API.
    const ordered: string[] = []
    for (const { chunk, url } of body.urls) {
      ordered[chunk] = url
    }

    cache.set(key, { urls: ordered, expiresAt: body.expires_at })

    return ordered
  } catch {
    return undefined
  }
}

/**
 * Presigned URLs for an authenticated file's active version.
 */
export async function fileChunkUrls(fileId: string): Promise<string[] | undefined> {
  return chunkUrls(`file:${fileId}`, `/api/storage/${fileId}/chunk-urls`, 'get')
}

/**
 * Presigned URLs for a public share link.
 */
export async function linkChunkUrls(linkId: string): Promise<string[] | undefined> {
  return chunkUrls(`link:${linkId}`, `/api/links/${linkId}/chunk-urls`, 'post')
}
