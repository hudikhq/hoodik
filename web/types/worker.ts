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
  transferableUploadedChunks: Uint32Array
  transferableFile: UploadAppFile

  /**
   * Presigned bucket URLs indexed by chunk, when the deployment serves them.
   *
   * Resolved on the main thread, where the session and the capability both
   * live. An index this does not cover is uploaded through the server.
   */
  directUrls?: string[]
}

/**
 * Message sent to the worker to download a file
 */
export type DownloadFileMessage = {
  transferableFile: AppFile

  /**
   * Presigned bucket URLs indexed by chunk, when the deployment serves them.
   *
   * Resolved on the main thread rather than here: the manifest is the one
   * place direct-versus-relayed is decided, and it is shared with every other
   * consumer of the same file. An index this does not cover is fetched
   * through the server.
   */
  directUrls?: string[]
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

  /**
   * Which part of the pipeline the file is in. `processing` covers the
   * gap between the last received byte and the blob reaching the
   * browser — final decrypt and the large buffer copies — which used to
   * read as a stall at 100%.
   */
  stage?: 'downloading' | 'processing'
}

/**
 * File returned after download to pipe into browser
 * download.
 */
export type DownloadCompletedResponseMessage = {
  transferableFile: AppFile
  blob: Blob
}
