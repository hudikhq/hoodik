/**
 * Session-spanning cache of encrypted thumbnail blobs in localStorage.
 *
 * Ciphertext only — the file key never touches disk, so a cached entry
 * is exactly as opaque as the server's copy. Storage-file keys carry the
 * file's `active_version` so an edit that replaces the thumbnail drops
 * the stale entry on the next read; link thumbnails are immutable
 * snapshots and cache without a version.
 *
 * The cache is best-effort: quota pressure evicts every cached
 * thumbnail and retries once, and any storage failure (e.g. Safari
 * private mode) silently falls back to the network path.
 */
const PREFIX = 'hoodik:thumbnail:'

function keyFor(id: string, version?: number): string {
  return version === undefined ? `${PREFIX}${id}` : `${PREFIX}${id}:${version}`
}

export function get(id: string, version?: number): string | null {
  try {
    return localStorage.getItem(keyFor(id, version))
  } catch {
    return null
  }
}

export function put(id: string, version: number | undefined, ciphertext: string): void {
  evict(id)

  try {
    localStorage.setItem(keyFor(id, version), ciphertext)
  } catch {
    evictAll()
    try {
      localStorage.setItem(keyFor(id, version), ciphertext)
    } catch {
      // Best-effort cache; the network path still works.
    }
  }
}

/** Drop every cached version of one thumbnail. */
export function evict(id: string): void {
  removeMatching((key) => key.startsWith(`${PREFIX}${id}`))
}

/** Drop every cached thumbnail — used to recover from quota pressure. */
export function evictAll(): void {
  removeMatching((key) => key.startsWith(PREFIX))
}

function removeMatching(matches: (key: string) => boolean): void {
  try {
    const stale: string[] = []
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i)
      if (key && matches(key)) stale.push(key)
    }
    stale.forEach((key) => localStorage.removeItem(key))
  } catch {
    // Best-effort cache; nothing to clean if storage is unavailable.
  }
}
