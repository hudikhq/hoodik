import type { ApiTransfer, ErrorResponse } from '@/stores/api'
import type { ListAppFile, UploadAppFile } from './file'
import type { FileMetadataJson } from '.'

export type UploadWorkerMessage = {
  file: string
  key: string
  isDone: boolean
}

/**
 * Message sent to the worker to upload a chunk
 */
export type UploadChunkMessage = {
  api: ApiTransfer
  transferableFile: UploadAppFile
  metadataJson: FileMetadataJson
  data: Uint8Array
  chunk: number
  attempt?: number
}

/**
 * Message sent to the worker to download a file
 */
export type DownloadFileMessage = {
  api: ApiTransfer
  transferableFile: ListAppFile
  metadataJson: FileMetadataJson
}

/**
 * Message sent FROM the worker with chunk progress
 */
export type UploadChunkResponseMessage = {
  transferableFile: UploadAppFile
  metadataJson: FileMetadataJson | null
  chunk: number
  attempt: number
  error?: Error | ErrorResponse<any> | string
}

/**
 * Message sent FROM the worker with file download progress
 */
export type DownloadProgressResponseMessage = {
  transferableFile: ListAppFile
  metadataJson: FileMetadataJson | null
  chunkBytes: number
  error?: Error | ErrorResponse<any> | string
}
