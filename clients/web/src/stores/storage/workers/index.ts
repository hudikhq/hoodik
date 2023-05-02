import {} from './../../api'
export * from './chunk'
export * from './download'

import Api from '@/stores/api'

import type {
  DownloadAppFile,
  DownloadFileMessage,
  UploadAppFile,
  UploadFileMessage
} from '@/types'

/**
 * Use service worker to upload a single chunk
 */
export async function pushUploadToWorker(file: UploadAppFile): Promise<void> {
  if (!file.metadata?.key) {
    throw new Error(`File ${file.id} is missing key`)
  }

  const transferableFile = {
    ...file,
    uploaded_chunks: undefined,
    metadata: undefined
  }

  const apiTransfer = new Api().toJson()

  window.UPLOAD.postMessage({
    type: 'upload-file',
    apiTransfer,
    message: {
      transferableFile,
      metadataJson: file.metadata.toJson()
    } as UploadFileMessage
  })
}

/**
 * Send file to start downloading on the worker
 */
export async function startFileDownload(file: DownloadAppFile): Promise<void> {
  if (!file.metadata?.key) {
    throw new Error(`File ${file.id} is missing key`)
  }

  const transferableFile = { ...file, metadata: undefined, uploaded_chunks: undefined }

  const apiTransfer = new Api().toJson()

  window.DOWNLOAD.postMessage({
    type: 'download-file',
    apiTransfer,
    message: {
      transferableFile,
      metadataJson: file.metadata.toJson()
    } as DownloadFileMessage
  })
}
