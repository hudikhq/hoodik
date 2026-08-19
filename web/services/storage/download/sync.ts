import Api from '../../api'
import * as logger from '!/logger'
import { TransferDownloader } from 'transfer'
import { fileChunkUrls } from './direct'
import { buildDownloader, type DownloadSpec } from './downloader'
import { downloadBytesInWorker, downloadWorkerReady } from './worker'

import type { DownloadProgressFunction, AppFile } from '../../../types'

/**
 * Everything the crate needs to fetch this file, resolved once.
 *
 * The manifest lookup lives here rather than at each call site so that the
 * worker and this thread are asking the same question of the same module —
 * which is the only reason they cannot end up on different transports.
 */
async function specFor(file: AppFile): Promise<DownloadSpec> {
  if (!file.key) {
    throw new Error('Cannot download file without key')
  }

  return {
    id: file.id,
    size: file.size || 0,
    chunks: file.chunks,
    cipher: file.cipher,
    key: file.key as Uint8Array,
    directUrls: await fileChunkUrls(file.id)
  }
}

/**
 * Build a wasm downloader on this thread. The crate owns the whole transfer —
 * HTTP, retries, ordering, decryption — so nothing derived from the plaintext
 * ever exists outside it until the result is handed back. Callers must
 * `free()` it (or go through the helpers below).
 *
 * The fallback for when there is no worker to do it instead: decrypting a
 * large file here stalls rendering for as long as it takes.
 */
async function fileDownloader(file: AppFile): Promise<TransferDownloader> {
  return buildDownloader(await specFor(file), new Api().toJson())
}

/**
 * Fetch through the download worker when there is one, and on this thread
 * when there is not.
 *
 * Both routes are the same crate reading the same manifest, so which one runs
 * changes where the work happens and nothing else. A worker that fails for its
 * own reasons — it died, it was never spawned — falls back rather than failing
 * the read.
 */
async function fetchBytes(file: AppFile, chunk?: number): Promise<Uint8Array> {
  const spec = await specFor(file)

  if (downloadWorkerReady()) {
    try {
      return await downloadBytesInWorker(spec, new Api().toJson(), chunk)
    } catch (err) {
      logger.warn('[download] worker could not serve the bytes, falling back:', err)
    }
  }

  const downloader = buildDownloader(spec, new Api().toJson())

  try {
    return chunk === undefined
      ? await downloader.download(() => {}, () => false)
      : await downloader.downloadChunk(chunk, undefined)
  } finally {
    downloader.free()
  }
}

/**
 * Adapt the crate's JSON progress protocol to a plain byte callback.
 */
function bytesFromProgress(onBytes?: (bytes: number) => void): (progressJson: string) => void {
  return (progressJson: string) => {
    if (!onBytes) return

    const progress = JSON.parse(progressJson)
    if (progress.type === 'download' && typeof progress.bytes_downloaded === 'number') {
      onBytes(progress.bytes_downloaded)
    }
  }
}

/**
 * Download the file content
 */
export async function downloadAndDecrypt(
  file: AppFile,
  onBytes?: (bytes: number) => void
): Promise<Uint8Array> {
  // Byte progress only exists on the local route; the worker reports it for
  // the transfers the queue owns, and these reads are short-lived enough that
  // a caller watching them gets one final number rather than none.
  if (downloadWorkerReady()) {
    const bytes = await fetchBytes(file)
    onBytes?.(bytes.length)
    return bytes
  }

  const downloader = await fileDownloader(file)

  try {
    return await downloader.download(bytesFromProgress(onBytes), () => false)
  } finally {
    downloader.free()
  }
}

/**
 * Create readable stream from downloading chunks and stream them
 * to download of the browser
 */
export async function downloadAndDecryptStream(file: AppFile, progress?: DownloadProgressFunction) {
  const chunks = [...new Array(file.chunks)].map((_, i) => i)

  const stream = new ReadableStream({
    start: async () => {
      if (progress) {
        await progress(file, 0)
      }
    },
    pull: async (controller) => {
      const chunk = chunks.shift()

      if (!chunk && chunk !== 0) {
        controller.close()
        return
      }

      const data = await downloadChunk(file, chunk as number)
      if (data) {
        if (progress) {
          await progress(file, data.length)
        }

        return controller.enqueue(data)
      } else {
        controller.close()
      }
    }
  })

  const response = new Response(stream)
  const url = window.URL.createObjectURL(await response.blob())
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = file.name
  anchor.click()
  window.URL.revokeObjectURL(url)
}

/**
 * Download single file chunk and decrypt it
 */
export async function downloadChunk(file: AppFile, chunk: number, signal?: AbortSignal): Promise<Uint8Array> {
  // The wasm fetch can't be aborted mid-chunk; progressive consumers call
  // per chunk, so honouring the signal between chunks is where it counts.
  if (signal?.aborted) {
    throw new DOMException('Download aborted', 'AbortError')
  }

  return fetchBytes(file, chunk)
}
