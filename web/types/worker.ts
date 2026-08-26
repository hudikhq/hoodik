import type { InnerValidationErrors } from '!/api'
import type { DownloadSpec } from '!/storage/download/downloader'
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
 * Ask the worker for decrypted bytes and get them back — the whole file, or a
 * single chunk of it.
 *
 * Separate from [[DownloadFileMessage]] because that one is fire-and-forget
 * and hands the browser a Blob at the end. These are ordinary calls with a
 * return value: previews, forks, re-indexing and version history all want the
 * plaintext in hand. `request` correlates the reply, so a video asking for its
 * next chunk cannot be answered with a fork's.
 */
export type DownloadBytesMessage = {
  request: string
  spec: DownloadSpec

  /** Set to fetch one chunk rather than the whole file. */
  chunk?: number
}

/**
 * Reply to a [[DownloadBytesMessage]]. Exactly one of `bytes` or `error`.
 */
export type DownloadBytesResponseMessage = {
  request: string
  bytes?: Uint8Array
  error?: WorkerErrorType
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
