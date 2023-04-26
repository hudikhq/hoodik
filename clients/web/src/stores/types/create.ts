export interface CreateFile {
  /**
   * Unencrypted file name
   */
  name: string

  /**
   * Mime type of the unencrypted file
   */
  mime: string

  /**
   * Size of the unencrypted file in bytes
   */
  size?: number

  /**
   * Number of chunks the file is split into
   */
  chunks?: number

  /**
   * If the file or directory is a child of another
   * directory this is where you put its id
   */
  file_id?: number

  /**
   * When was the file created on disk
   */
  file_created_at?: string
}

export interface EncryptedCreateFile {
  /**
   * JSON stringified metadata that was encrypted
   * with the users public key
   */
  encrypted_metadata?: string

  /**
   * Tokenize the unencrypted file name or any search data,
   * hash each token and load it in this array.
   */
  search_tokens_hashed?: string[]

  /**
   * Unencrypted name hash
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
   * Number of chunks the file is split into
   */
  chunks?: number

  /**
   * If the file or directory is a child of another
   * directory this is where you put its id
   */
  file_id?: number

  /**
   * When was the file created on disk
   */
  file_created_at?: string
}
