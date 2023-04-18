import Api, { type Query } from '../api'
import type { Key } from '../cryptfns/aes'
import * as cryptfns from '../cryptfns'
import type { KeyPair } from '../cryptfns/rsa'

export class FileMetadata {
  /**
   * File name
   */
  name?: string

  /**
   * AES key used to encrypt the file data
   */
  key?: Key;

  /**
   * Place to store any other possible metadata
   */
  [other: string]: any

  constructor(name: string, key: Key | undefined) {
    this.name = name
    this.key = key
  }

  /**
   * Set any other possible arguments of the metadata
   */
  set(key: string, value: any): void {
    if (key !== 'name' && key !== 'key') {
      this[key] = value
    }
  }

  /**
   * Return encrypted string of the file metadata
   */
  async encrypt(publicKey: string): Promise<string> {
    let key

    if (this.key) {
      key = cryptfns.aes.keyToStringJson(this.key)
    }

    return cryptfns.rsa.encryptMessage(JSON.stringify({ ...this, key }), publicKey)
  }

  /**
   * Decrypt the file metadata from string
   */
  static async decrypt(encrypted: string, keypair: cryptfns.rsa.KeyPair): Promise<FileMetadata> {
    const decrypted = await cryptfns.rsa.decryptMessage(keypair, encrypted)

    try {
      const obj = JSON.parse(decrypted)
      let key = obj.key

      if (key) {
        try {
          key = cryptfns.aes.keyFromStringJson(obj.key)
        } catch (e) {
          obj.error =
            'Data key is not valid, most likely the file encryption was broken and it is unrecoverable'
        }
      }

      const metadata = new FileMetadata(obj.name, key)

      for (const key in obj) {
        if (key !== 'name' && key !== 'key') {
          metadata[key] = obj[key]
        }
      }

      return metadata
    } catch (error) {
      return new FileMetadata('decrypt failed', undefined)
    }
  }
}

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

export interface AppFile extends EncryptedAppFile {
  /**
   * Unencrypted file metadata
   */
  metadata?: FileMetadata

  /**
   * Decrypted data of the file
   */
  data?: Uint8Array
}

export interface EncryptedAppFile {
  id: number

  /**
   * User id of the user that loaded the file
   */
  user_id: number

  /**
   * Is the current user file owner
   */
  is_owner: boolean

  /**
   * Encrypted file metadata
   */
  encrypted_metadata: string

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
  file_id?: number

  /**
   * This is an optional field that can be
   * set to the original file creation date
   */
  file_created_at: string

  /**
   * Database file creation date
   */
  created_at: string

  /**
   * Date of the last uploaded chunk
   */
  finished_upload_at?: string

  /**
   * Lets us know if the file was newly created or was
   * already in the database
   */
  is_new: boolean

  /**
   * List of chunks that were uploaded
   * by their chunk number from 0 to chunks - 1
   */
  uploaded_chunks?: number[]
}

export interface Parameters extends Query {
  dir_id?: number | null
  order?: 'asc' | 'desc'
  order_by?: 'created_at' | 'size'
}

export interface FileResponse {
  parents?: AppFile[]
  children: AppFile[]
}

/**
 * Create a file or directory on the server
 */
export async function create(keypair: KeyPair, unencrypted: CreateFile): Promise<AppFile> {
  const key = cryptfns.aes.generateKey()
  const metadata = new FileMetadata(unencrypted.name, key)

  // TODO: tokenize the name and search data
  const search_tokens_hashed = [].map((token) => cryptfns.sha256.digest(token))

  const createFile: EncryptedCreateFile = {
    search_tokens_hashed,
    name_hash: cryptfns.sha256.digest(unencrypted.name),
    encrypted_metadata: await metadata.encrypt(keypair.publicKey as string),
    mime: unencrypted.mime,
    size: unencrypted.size,
    chunks: unencrypted.chunks,
    file_id: unencrypted.file_id,
    file_created_at: unencrypted.file_created_at
  }

  const response = await Api.post<EncryptedCreateFile, AppFile>(
    '/api/storage',
    undefined,
    createFile
  )

  if (!response?.body?.id) {
    throw new Error('Failed to create file')
  }

  const file = response.body

  return { ...file, metadata }
}

/**
 * Get file or directory metadata
 */
export async function get(keypair: KeyPair, file_id: number): Promise<AppFile> {
  const response = await Api.get<AppFile>(`/api/storage/${file_id}/metadata`, undefined)

  if (!response?.body?.id) {
    throw new Error('Failed to get file or directory')
  }

  const file = response.body
  const metadata = await FileMetadata.decrypt(file.encrypted_metadata, keypair)

  return { ...file, metadata }
}

/**
 *  Lookup directory or file by its name and parent directory
 */
export async function getByName(
  keypair: KeyPair,
  name: string,
  parent_id?: number
): Promise<AppFile> {
  const nameHash = cryptfns.sha256.digest(name)

  if (parent_id !== undefined || typeof parent_id !== 'number') {
    parent_id = undefined
  }

  const response = await Api.get<AppFile>(`/api/storage/${nameHash}/name-hash`, { parent_id })

  if (!response?.body?.id) {
    throw new Error('Failed to get file or directory')
  }

  const file = response.body
  const metadata = await FileMetadata.decrypt(file.encrypted_metadata, keypair)

  return { ...file, metadata }
}

/**
 * Get file or directory metadata
 */
export async function find(parameters: Parameters): Promise<FileResponse> {
  // @ts-ignore
  if (isNaN(parameters.dir_id)) {
    delete parameters.dir_id
  }

  console.log(parameters)
  const response = await Api.get<FileResponse>(`/api/storage`, parameters)

  return response.body || { children: [], parents: [] }
}

/**
 * Get file or directory metadata
 */
export async function remove(fileId: number): Promise<void> {
  await Api.delete(`/api/storage/${fileId}`)
}
