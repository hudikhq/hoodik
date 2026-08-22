import Api from '../../api'
import * as logger from '!/logger'
import { evictChunkUrls, fileChunkUrls } from './direct'
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
    directUrls: await fileChunkUrls(file.id, file.active_version)
  }
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

  try {
    return await runDownloader(spec, chunk)
  } catch (err) {
    // A manifest can outlive the content it described: another session — or a
    // share editor — replaces the file, and the cached URLs keep pointing at
    // the old version's chunks until they expire days later. Every read would
    // fail the same way for the rest of the session, so drop the manifest and
    // serve this read through the server instead.
    if (!spec.directUrls) throw err

    logger.warn('[download] direct manifest failed, evicting it and relaying:', err)
    evictChunkUrls(file.id)

    return runDownloader({ ...spec, directUrls: undefined }, chunk)
  }
}

async function runDownloader(
  spec: DownloadSpec,
  chunk?: number,
  onBytes?: (bytes: number) => void
): Promise<Uint8Array> {
  const downloader = buildDownloader(spec, new Api().toJson())

  try {
    return chunk === undefined
      ? await downloader.download(bytesFromProgress(onBytes), () => false)
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

  const spec = await specFor(file)

  try {
    return await runDownloader(spec, undefined, onBytes)
  } catch (err) {
    // Same healing as `fetchBytes`: a stale manifest must not pin every read
    // of this file to the same failure until its URLs expire.
    if (!spec.directUrls) throw err

    logger.warn('[download] direct manifest failed, evicting it and relaying:', err)
    evictChunkUrls(file.id)

    return runDownloader({ ...spec, directUrls: undefined }, undefined, onBytes)
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
