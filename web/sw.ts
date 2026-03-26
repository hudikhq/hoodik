import {
  TransferUploader,
  TransferDownloader,
  aes_encrypt,
  aes_decrypt
} from './node_modules/transfer/transfer.js'
import * as logger from '!/logger'

import type { ApiTransfer, ErrorResponse } from './services/api'
import type {
  DownloadCompletedResponseMessage,
  DownloadFileMessage,
  DownloadProgressResponseMessage,
  UploadAppFile,
  UploadChunkResponseMessage,
  UploadFileMessage,
  WorkerErrorType
} from './types'

logger.debug(`[sw] Worker initialized (${self.name || 'unnamed'})`)

self.canceled = {
  upload: [],
  download: []
}

onmessage = async (message: MessageEvent<any>) => {
  const msgType = message.data?.type || 'unknown'
  if (!String(msgType).includes('upload')) {
    logger.debug(`[sw:${self.name || '?'}] received message:`, msgType)
  }

  if (message.data?.type === 'ping') {
    self.__IDENTITY = message.data?.name || undefined
    postMessage({ type: 'pong' })
  }

  if (message.data?.type === 'encrypt' || message.data?.type === 'decrypt') {
    handleCrypto(
      message.data.type,
      message.data.message.id,
      message.data.message.data,
      message.data.message.key
    )
  }

  if (message.data?.type === 'cancel') {
    const type = message.data.kind
    if (type === 'upload') {
      self.canceled.upload.push(message.data.id)
    } else if (type === 'download') {
      self.canceled.download.push(message.data.id)
    }
  }

  if (message.data?.type === 'upload-file') {
    handleUploadFile(message.data.apiTransfer, message.data.message)
  }

  if (message.data?.type === 'download-file') {
    handleDownloadFile(message.data.apiTransfer, message.data.message)
  }
}

async function handleCrypto(
  type: 'encrypt' | 'decrypt',
  id: string,
  data: Uint8Array,
  key: Uint8Array
) {
  logger.debug('In worker, handling crypto', type, id)

  const result = type === 'encrypt' ? aes_encrypt(key, data) : aes_decrypt(key, data)

  logger.debug('In worker, handling crypto, posting message', type, id)

  postMessage({
    type,
    message: { id, result }
  })
}

async function handleUploadFile(
  apiTransfer: ApiTransfer,
  { transferableUploadedChunks, transferableFile }: UploadFileMessage
) {
  const file = transferableFile as UploadAppFile
  const t0 = performance.now()
  const uploadedSet = new Set<number>(file.uploaded_chunks || [])
  const startedAt = file.started_upload_at || new Date().toISOString()

  // Emit an immediate "upload started" progress so the UI updates right away
  // (the transfer layer only reports progress on completed chunk uploads).
  postMessage({
    type: 'upload-progress',
    response: {
      transferableFile: {
        ...file,
        uploaded_chunks: Array.from(uploadedSet),
        chunks_stored: uploadedSet.size,
        started_upload_at: startedAt,
        last_progress_at: new Date().toISOString()
      },
      chunk: 0,
      attempt: 0,
      isDone: false
    } as UploadChunkResponseMessage
  })

  let uploader: TransferUploader | undefined
  try {
    const baseUrl = apiTransfer.apiUrl || ''
    const jwtToken = apiTransfer.jwtToken || undefined
    const refreshToken = apiTransfer.refreshToken || undefined
    const cipher = file.cipher

    // Disable all hashes in WASM — SHA-256 is computed by the top-level HASH worker
    // spawned from main.ts and reported back through queue.ts independently.
    const hashDisableMask = 1 | 2 | 4 | 8

    const reportUploadProgress = (progressJson: string) => {
      const progress = JSON.parse(progressJson)

      if (progress.type === 'upload' && progress.chunk !== undefined) {
        uploadedSet.add(progress.chunk)
      }

      postMessage({
        type: 'upload-progress',
        response: {
          transferableFile: {
            ...file,
            uploaded_chunks: Array.from(uploadedSet),
            chunks_stored: uploadedSet.size,
            started_upload_at: startedAt,
            last_progress_at: new Date().toISOString()
          },
          chunk: progress.chunk,
          attempt: 0,
          isDone: progress.is_done,
          error: progress.type === 'error' ? { context: progress.error } : undefined
        } as UploadChunkResponseMessage
      })
    }

    uploader = new TransferUploader(
      file.id,
      baseUrl,
      jwtToken,
      refreshToken,
      file.key as Uint8Array
    )
    uploader.set_uploaded_chunks(new Uint32Array(transferableUploadedChunks || []))
    uploader.set_hash_mask(hashDisableMask)
    uploader.set_cipher(cipher)

    await uploader.upload(
      file.file as File,
      undefined,
      reportUploadProgress,
      (fileId: string) => self.canceled?.upload?.includes(fileId) ?? false
    )
    uploader.free()
    uploader = undefined

    logger.info(`[sw:upload] "${file.name}" completed in ${(performance.now() - t0).toFixed(0)}ms`)
  } catch (err) {
    try { uploader?.free() } catch { /* ignore errors during cleanup */ }
    uploader = undefined
    logger.error(
      `[sw:upload] "${file.name}" failed after ${(performance.now() - t0).toFixed(0)}ms:`,
      err
    )
    postMessage({
      type: 'upload-progress',
      response: {
        transferableFile: file,
        attempt: 0,
        isDone: false,
        error: handleError(err as Error)
      } as UploadChunkResponseMessage
    })
  } finally {
    const idx = self.canceled.upload.indexOf(file.id)
    if (idx !== -1) self.canceled.upload.splice(idx, 1)
  }
}

async function handleDownloadFile(
  apiTransfer: ApiTransfer,
  { transferableFile }: DownloadFileMessage
) {
  const t0 = performance.now()
  logger.info(
    `[sw:download] starting "${transferableFile.name}" (${transferableFile.id}), ${transferableFile.chunks} chunks`
  )

  let downloader: TransferDownloader | undefined
  try {
    const baseUrl = apiTransfer.apiUrl || ''
    const jwtToken = apiTransfer.jwtToken || undefined
    const refreshToken = apiTransfer.refreshToken || undefined

    const cipher = transferableFile.cipher

    downloader = new TransferDownloader(
      transferableFile.id,
      transferableFile.size || 0,
      transferableFile.chunks || 0,
      baseUrl,
      jwtToken,
      refreshToken,
      transferableFile.key as Uint8Array
    )
    downloader.set_cipher(cipher)
    const bytes = await downloader.download(
      (progressJson: string) => {
        const progress = JSON.parse(progressJson)

        if (progress.type === 'download') {
          logger.debug(
            `[sw:download] "${transferableFile.name}" progress: ${progress.bytes_downloaded} bytes`
          )
          postMessage({
            type: 'download-progress',
            response: {
              transferableFile,
              chunkBytes: progress.bytes_downloaded
            } as DownloadProgressResponseMessage
          })
        }
      },
      (fileId: string) => self.canceled?.download?.includes(fileId) ?? false
    )
    downloader.free()
    downloader = undefined

    logger.info(
      `[sw:download] "${transferableFile.name}" completed in ${(performance.now() - t0).toFixed(
        0
      )}ms, ${bytes.length} bytes`
    )

    const blob = new Blob([bytes])

    postMessage({
      type: 'download-completed',
      response: {
        transferableFile,
        blob
      } as DownloadCompletedResponseMessage
    })
  } catch (err) {
    try { downloader?.free() } catch { /* ignore errors during cleanup */ }
    downloader = undefined
    logger.error(
      `[sw:download] "${transferableFile.name}" failed after ${(performance.now() - t0).toFixed(
        0
      )}ms:`,
      err
    )
    const error = err as ErrorResponse<unknown>

    postMessage({
      type: 'download-progress',
      response: {
        transferableFile,
        chunkBytes: 0,
        error: handleError(error)
      } as DownloadProgressResponseMessage
    })
  } finally {
    const idx = self.canceled.download.indexOf(transferableFile.id)
    if (idx !== -1) self.canceled.download.splice(idx, 1)
  }
}

function handleError(error?: undefined | Error | ErrorResponse<any> | string): WorkerErrorType {
  if (!error) return

  if (typeof error === 'string') {
    return { context: error }
  }

  if (error instanceof Error) {
    return {
      context:
        (error as ErrorResponse<unknown>).validation ||
        (error as ErrorResponse<unknown>).description ||
        error.message,
      stack: error.stack
    }
  }
}
