import type { InnerValidationErrors } from '!/api'
import type { AppFile, UploadAppFile } from './file'

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
  transferableUploadedChunks: Uint16Array
  transferableFile: UploadAppFile
}

/**
 * Message sent to the worker to download a file
 */
export type DownloadFileMessage = {
  transferableFile: AppFile
}

/**
 * Message sent FROM the worker with chunk progress
 */
export type UploadChunkResponseMessage = {
  transferableFile: UploadAppFile
  chunk: number
  attempt: number
  isDone?: boolean
  error?: WorkerErrorType
}

/**
 * Message sent FROM the worker with file download progress
 */
export type DownloadProgressResponseMessage = {
  transferableFile: AppFile
  chunkBytes: number
  error?: WorkerErrorType
}

/**
 * File returned after download to pipe into browser
 * download.
 */
export type DownloadCompletedResponseMessage = {
  transferableFile: AppFile
  blob: Blob
}
