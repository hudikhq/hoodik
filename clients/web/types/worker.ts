import type { InnerValidationErrors } from '!/api'
import type { ListAppFile, UploadAppFile } from './file'
import type { FileMetadataJson } from '.'

/**
 * Message error that is sent from the worker
 */
export type WorkerErrorType =
  | undefined
  | { context: InnerValidationErrors | string | undefined; stack?: string }

/**
 * Message sent to the worker to upload a file,
 * the worker takes care of the chunking and sending
 * one by one
 */
export type UploadFileMessage = {
  transferableFile: UploadAppFile
  metadataJson: FileMetadataJson
}

/**
 * Message sent to the worker to download a file
 */
export type DownloadFileMessage = {
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
  isDone?: boolean
  error?: WorkerErrorType
}

/**
 * Message sent FROM the worker with file download progress
 */
export type DownloadProgressResponseMessage = {
  transferableFile: ListAppFile
  metadataJson: FileMetadataJson | null
  chunkBytes: number
  error?: WorkerErrorType
}

/**
 * File returned after download to pipe into browser
 * download.
 */
export type DownloadCompletedResponseMessage = {
  transferableFile: ListAppFile
  metadataJson: FileMetadataJson | null
  blob: Blob
}
