import type { EncryptedLink } from './links'
import type { WorkerErrorType } from './worker'

export interface DeleteManyFiles {
  ids: string[]
}

export interface MoveManyFiles {
  ids: string[]
  file_id?: string | null | undefined
}

export interface UploadAppFile extends AppFile {
  file: File
  started_upload_at?: string
  last_progress_at?: string
  error?: WorkerErrorType
  cancel?: boolean
}

export interface DownloadAppFile extends AppFile {
  started_download_at?: string
  finished_downloading_at?: string
  error?: WorkerErrorType
  cancel?: boolean
  downloadedBytes?: number
}

export interface AppFile extends EncryptedAppFile, AppFileUnencryptedPart {
  data?: Uint8Array

  /**
   * Client-only id that follows a file across worker/UI boundaries before
   * the server assigns the real one, so it doesn't show up twice in lists.
   */
  temporaryId?: string
}

export interface EncryptedAppFile extends AppFileEncryptedPart {
  id: string
  user_id: string
  is_owner: boolean
  name_hash: string
  mime: string
  size?: number

  /** `Math.ceil(size / CHUNK_SIZE_BYTES)`. */
  chunks: number

  chunks_stored?: number

  /** Parent directory id; null/undefined means the user's root. */
  file_id?: string

  /** Client-supplied override for the original file's mtime. */
  file_modified_at: number

  created_at: number

  /** Timestamp of the final chunk upload — presence means finalize fired. */
  finished_upload_at?: number

  /** True only in the response to `createFile`; false on subsequent fetches. */
  is_new: boolean

  md5?: string
  sha1?: string
  sha256?: string
  blake2b?: string

  /** Whether this file's content can be replaced (markdown notes). */
  editable: boolean

  /**
   * Version of the chunks readers should fetch. Always set; defaults to 1.
   * Increments on every successful edit so clients can cache-bust.
   */
  active_version: number

  /**
   * Set while a save is in flight — chunks are landing into `v{pending_version}/`.
   * Atomically swapped to `active_version` on finalize.
   */
  pending_version?: number

  /**
   * Total chunks expected for the in-flight upload (undefined when none).
   * Auto-finalize fires when `chunks_stored` matches this.
   */
  pending_chunks?: number

  pending_size?: number

  /** Indices 0..chunks-1 of already-stored chunks; used for resume. */
  uploaded_chunks?: number[]

  link?: EncryptedLink
}

/**
 * Historical snapshot of an editable file. Returned by
 * `GET /api/storage/{file_id}/versions`. The active version is NOT in
 * this list — it lives on the file row itself.
 */
export interface FileVersion {
  id: string
  file_id: string
  version: number
  /** UUID of the user who saved, or null for anonymous link saves. */
  user_id: string | null
  /** True for anonymous saves through a shared editable link (A4). */
  is_anonymous: boolean
  size: number
  chunks: number
  /** Per-version sha256 for exact restore. */
  sha256: string | null
  /** Unix seconds — time the version was archived. */
  created_at: number
}

export interface AppFileUnencryptedPart {
  key?: Uint8Array
  name: string
  thumbnail?: string
}

export interface AppFileEncryptedPart {
  /** Symmetric file key wrapped with the owner's RSA public key. */
  encrypted_key: string

  encrypted_name: string
  encrypted_thumbnail?: string

  /** Cipher used for both chunk payloads and metadata (name, thumbnail). */
  cipher: string
}
