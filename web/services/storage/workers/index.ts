import Api from '../../api'
import * as logger from '!/logger'

import type { DownloadAppFile, DownloadFileMessage, UploadAppFile, UploadFileMessage } from 'types'

/**
 * Push file to the upload worker (transfer WASM handles chunking, encryption, HTTP)
 */
export async function pushUploadToWorker(file: UploadAppFile): Promise<void> {
  if (!file.key) {
    throw new Error(`File ${file.id} is missing key`)
  }

  logger.debug(`[worker:upload] posting "${file.name}" (${file.id}) to UPLOAD worker`)

  const transferableFile = {
    ...file,
    uploaded_chunks: undefined
  }

  const apiTransfer = new Api().toJson()
  const transferableUploadedChunks = new Uint32Array(file.uploaded_chunks || [])

  window.UPLOAD.postMessage({
    type: 'upload-file',
    apiTransfer,
    message: {
      transferableUploadedChunks,
      transferableFile
    } as UploadFileMessage
  })

  // Start SHA-256 hashing in the dedicated top-level hash worker (runs in parallel with upload).
  if (window.HASH && file.file instanceof File) {
    logger.debug(`[worker:hash] posting hash-file for "${file.name}" (${file.id})`)
    window.HASH.postMessage({ type: 'hash-file', id: file.id, file: file.file })
  } else {
    logger.warn(`[worker:hash] SKIPPING hash for "${file.name}" — HASH worker: ${!!window.HASH}, file instanceof File: ${file.file instanceof File}`)
  }
}

/**
 * Push file to the download worker (transfer WASM handles chunking, decryption, HTTP)
 */
export async function startFileDownload(file: DownloadAppFile): Promise<void> {
  if (!file.key) {
    throw new Error(`File ${file.id} is missing key`)
  }

  logger.debug(`[worker:download] posting "${file.name}" (${file.id}) to DOWNLOAD worker`)

  const transferableFile = {
    ...file,
    uploaded_chunks: undefined,
    link: undefined
  }

  const apiTransfer = new Api().toJson()

  window.DOWNLOAD.postMessage({
    type: 'download-file',
    apiTransfer,
    message: {
      transferableFile
    } as DownloadFileMessage
  })
}
