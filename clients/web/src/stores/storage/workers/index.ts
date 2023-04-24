import Api from '@/stores/api'

export * from './chunk'
export * from './download'

import type {
  DownloadAppFile,
  DownloadFileMessage,
  UploadAppFile,
  UploadChunkMessage
} from '../types'

/**
 * Use service worker to upload a single chunk
 */
export async function pushUploadToWorker(file: UploadAppFile): Promise<void> {
  if (!file.metadata?.key) {
    throw new Error(`File ${file.id} is missing key`)
  }

  const transferableFile = { ...file, metadata: undefined }

  window.SW.postMessage({
    type: 'upload-file',
    message: {
      api: new Api().toJson(),
      transferableFile,
      metadataJson: file.metadata.toJson()
    } as UploadChunkMessage
  })
}

/**
 * Use service worker to upload a single chunk
 */
export async function pushChunkToWorker(
  file: UploadAppFile,
  data: Uint8Array,
  chunk: number
): Promise<void> {
  if (!file.metadata?.key) {
    throw new Error(`File ${file.id} is missing key`)
  }

  const transferableFile = { ...file, metadata: undefined }

  window.SW.postMessage({
    type: 'upload-chunk',
    message: {
      api: new Api().toJson(),
      transferableFile,
      metadataJson: file.metadata.toJson(),
      data,
      chunk
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

  const transferableFile = { ...file, metadata: undefined }

  window.SW.postMessage({
    type: 'download-file',
    message: {
      api: new Api().toJson(),
      transferableFile,
      metadataJson: file.metadata.toJson()
    } as DownloadFileMessage
  })
}
