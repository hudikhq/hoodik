import type { EncryptedLink } from './links'
import type { WorkerErrorType } from './worker'

/**
 * Delete many files at once
 */
export interface DeleteManyFiles {
  ids: string[]
}

/**
 * Move many files to a new directory
 */
export interface MoveManyFiles {
  ids: string[]
  file_id?: string | null | undefined
}

export interface UploadAppFile extends AppFile {
  /**
   * File system file
   */
  file: File

  /**
   * Start of the upload
   */
  started_upload_at?: string

  /**
   * Last progress report
   */
  last_progress_at?: string

  /**
   * Possible error while uploading the file
   */
  error?: WorkerErrorType

  /**
   * Signalize the file to cancel the upload
   */
  cancel?: boolean
}

export interface DownloadAppFile extends AppFile {
  /**
   * Start of the download
   */
  started_download_at?: string

  /**
   * Finish of the file downloading
   */
  finished_downloading_at?: string

  /**
   * Possible error while uploading the file
   */
  error?: WorkerErrorType

  /**
   * Signalize the file to cancel the upload
   */
  cancel?: boolean

  /**
   * Number of bytes downloaded so far
   */
  downloadedBytes?: number
}

export interface AppFile extends EncryptedAppFile, AppFileUnencryptedPart {
  /**
   * Decrypted data of the file
   */
  data?: Uint8Array

  /**
   * Temporary ID that is only used within the client application
   * it helps us keep track of the file while its moving through
   * various process methods so we don't duplicate it in the UI
   */
  temporaryId?: string
}

export interface EncryptedAppFile extends AppFileEncryptedPart {
  id: string

  /**
   * User id of the user that loaded the file
   */
  user_id: string

  /**
   * Is the current user file owner
   */
  is_owner: boolean

  /**
   * Unencrypted file name hash
   */
  name_hash: string

  /**
   * Mime type of the unencrypted file
   */
  mime: string

  /**
   * Size of the unencrypted file in bytes
   */
  size?: number

  /**
   * Number of chunks the file is split into,
   * this is Math.ceil(size / CHUNK_SIZE_BYTES)
   */
  chunks: number

  /**
   * Number of chunks that were uploaded
   */
  chunks_stored?: number

  /**
   * If the file or directory is a child of another
   * directory then this will be the parent directory id
   */
  file_id?: string

  /**
   * This is an optional field that can be
   * set to the original file creation date
   */
  file_modified_at: number

  /**
   * Database file creation date
   */
  created_at: number

  /**
   * Date of the last uploaded chunk
   */
  finished_upload_at?: number

  /**
   * Lets us know if the file was newly created or was
   * already in the database
   */
  is_new: boolean

  /**
   * MD5 hash of the file (if it is uploaded, and a file)
   */
  md5?: string

  /**
   * SHA1 hash of the file (if it is uploaded, and a file)
   */
  sha1?: string

  /**
   * SHA256 hash of the file (if it is uploaded, and a file)
   */
  sha256?: string

  /**
   * SHA512 hash of the file (if it is uploaded, and a file)
   */
  blake2b?: string

  /**
   * List of chunks that were uploaded
   * by their chunk number from 0 to chunks - 1
   */
  uploaded_chunks?: number[]

  /**
   * File shared public link (if it exists)
   */
  link?: EncryptedLink
}

/**
 * Unencrypted file parts
 */
export interface AppFileUnencryptedPart {
  /**
   * Decrypted file key
   */
  key?: Uint8Array

  /**
   * Decrypted file name
   */
  name: string

  /**
   * Decrypted file thumbnail
   */
  thumbnail?: string
}

/**
 * Encrypted file parts
 */
export interface AppFileEncryptedPart {
  /**
   * Encrypted file metadata
   */
  encrypted_key: string

  /**
   * Encrypted file name
   */
  encrypted_name: string

  /**
   * Encrypted file thumbnail
   */
  encrypted_thumbnail?: string
}
