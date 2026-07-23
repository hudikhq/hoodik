import Api from '../../api'
import { TransferDownloader } from 'transfer'

import type { DownloadProgressFunction, AppFile } from '../../../types'

/**
 * Build a wasm downloader for an authenticated storage file. The crate owns
 * the whole transfer — HTTP, retries, ordering, decryption — so nothing
 * derived from the plaintext ever exists outside it until the result is
 * handed back. Callers must `free()` it (or go through the helpers below).
 */
function fileDownloader(file: AppFile): TransferDownloader {
  if (!file.key) {
    throw new Error('Cannot download file without key')
  }

  const { apiUrl, jwtToken, refreshToken } = new Api().toJson()
  const downloader = new TransferDownloader(
    file.id,
    file.size || 0,
    file.chunks,
    apiUrl || '',
    jwtToken || undefined,
    refreshToken || undefined,
    file.key as Uint8Array
  )
  downloader.set_cipher(file.cipher)

  return downloader
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
  const downloader = fileDownloader(file)

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

  const downloader = fileDownloader(file)

  try {
    return await downloader.downloadChunk(chunk, undefined)
  } finally {
    downloader.free()
  }
}
