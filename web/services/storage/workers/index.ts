export * from './uploadChunk'
export * from './downloadChunks'
// export * from './download'

import Api from '../../api'

import type { DownloadAppFile, DownloadFileMessage, UploadAppFile, UploadFileMessage } from 'types'

/**
 * Use service worker to upload a single chunk
 */
export async function pushUploadToWorker(file: UploadAppFile): Promise<void> {
  if (!file.key) {
    throw new Error(`File ${file.id} is missing key`)
  }

  const transferableFile = {
    ...file,
    uploaded_chunks: undefined
  }

  const apiTransfer = new Api().toJson()
  const transferableUploadedChunks = new Uint16Array(file.uploaded_chunks || [])

  window.UPLOAD.postMessage({
    type: 'upload-file',
    apiTransfer,
    message: {
      transferableUploadedChunks,
      transferableFile
    } as UploadFileMessage
  })
}

/**
 * Send file to start downloading on the worker
 */
export async function startFileDownload(file: DownloadAppFile): Promise<void> {
  if (!file.key) {
    throw new Error(`File ${file.id} is missing key`)
  }

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
