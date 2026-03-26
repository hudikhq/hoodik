/**
 * Dedicated top-level Web Worker: computes SHA-256 using the browser's native
 * SubtleCrypto API — no WASM required.
 *
 * Started from main.ts alongside the UPLOAD and DOWNLOAD workers.
 *
 * Protocol:
 *   → { type: 'hash-file', id: string, file: File }
 *       Read the file, compute SHA-256, and reply
 *       { type: 'hash-done', id, sha256 } or { type: 'hash-error', id, error }.
 */

console.debug('[hash-worker] initialized')

self.onmessage = async (e: MessageEvent) => {
  const { type, id, file } = e.data as {
    type: string
    id: string
    file: File
  }

  if (type !== 'hash-file') return

  console.debug(`[hash-worker] hashing "${file?.name}" id=${id} size=${file?.size}`)

  try {
    const buffer = await file.arrayBuffer()
    const hashBuffer = await crypto.subtle.digest('SHA-256', buffer)
    const hex = Array.from(new Uint8Array(hashBuffer))
      .map((b) => b.toString(16).padStart(2, '0'))
      .join('')

    console.debug(`[hash-worker] done id=${id} sha256=${hex.slice(0, 8)}...`)
    self.postMessage({ type: 'hash-done', id, sha256: hex })
  } catch (err) {
    console.error('[hash-worker] error:', err)
    self.postMessage({ type: 'hash-error', id, error: err instanceof Error ? err.message : String(err) })
  }
}
