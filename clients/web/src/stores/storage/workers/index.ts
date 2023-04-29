export * from './chunk'
export * from './download'

import type {
  DownloadAppFile,
  DownloadFileMessage,
  UploadAppFile,
  UploadChunkMessage
} from '../../../types'

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

  window.UPLOAD.postMessage({
    type: 'upload-file',
    message: {
      transferableFile,
      metadataJson: file.metadata.toJson()
    } as UploadChunkMessage
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

  window.DOWNLOAD.postMessage({
    type: 'download-file',
    message: {
      transferableFile,
      metadataJson: file.metadata.toJson()
    } as DownloadFileMessage
  })
}
