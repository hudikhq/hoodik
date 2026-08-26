import * as logger from '!/logger'

import type { DownloadSpec } from './downloader'
import type { ApiTransfer } from '../../api'
import type { DownloadBytesResponseMessage } from '../../../types'

/**
 * Requests waiting on the download worker, keyed by the id sent with them.
 *
 * A message port has no notion of a call, so the correlation has to live
 * somewhere. Without it a video asking for its next chunk could be handed a
 * fork's, which is the kind of bug that shows up as one corrupt frame.
 */
const waiting = new Map<
  string,
  { resolve: (bytes: Uint8Array) => void; reject: (error: Error) => void }
>()

let counter = 0

/**
 * Whether the download worker is up and can be asked for bytes.
 *
 * False in a browser without `Worker`, in tests, and in the window between
 * boot and the worker attaching. Callers fall back to fetching on this thread,
 * which is the same transport by a slower route — not a different one.
 */
export function downloadWorkerReady(): boolean {
  return typeof window !== 'undefined' && 'DOWNLOAD' in window && !!window.DOWNLOAD
}

/**
 * Hand a reply to whoever is waiting for it. Called by the queue's worker
 * listener, which owns `onmessage`.
 */
export function resolveDownloadBytes(response: DownloadBytesResponseMessage): void {
  const pending = waiting.get(response.request)
  if (!pending) return

  waiting.delete(response.request)

  if (response.error || !response.bytes) {
    const context = response.error?.context
    pending.reject(
      new Error(typeof context === 'string' ? context : 'Download failed in the worker')
    )
    return
  }

  pending.resolve(response.bytes)
}

/**
 * Fail every outstanding request. The worker died, and a promise that never
 * settles is worse than one that rejects.
 */
export function failPendingDownloads(reason: string): void {
  for (const [, pending] of waiting) {
    pending.reject(new Error(reason))
  }
  waiting.clear()
}

/**
 * Ask the worker to fetch and decrypt, and wait for the bytes.
 *
 * Pass `chunk` for a single chunk rather than the whole file. The crate owns
 * the transfer either way — HTTP, retries, ordering, decryption — so nothing
 * derived from the plaintext exists outside the worker until the result comes
 * back.
 */
export function downloadBytesInWorker(
  spec: DownloadSpec,
  api: ApiTransfer,
  chunk?: number
): Promise<Uint8Array> {
  const request = `${spec.id}:${chunk ?? 'all'}:${counter++}`

  return new Promise<Uint8Array>((resolve, reject) => {
    waiting.set(request, { resolve, reject })

    try {
      window.DOWNLOAD.postMessage({
        type: 'download-bytes',
        apiTransfer: api,
        message: { request, spec, chunk }
      })
    } catch (err) {
      waiting.delete(request)
      logger.error('[worker:download] could not post a bytes request:', err)
      reject(err as Error)
    }
  })
}
