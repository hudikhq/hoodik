import type { Rename, EncryptedRename } from 'types'
import Api from '../api'
import * as cryptfns from '../cryptfns'

import type {
  AppFile,
  CreateFile,
  EncryptedCreateFile,
  FileResponse,
  Parameters,
  KeyPair,
  EncryptedAppFile,
  SearchQuery,
  AppFileEncryptedPart,
  AppFileUnencryptedPart,
  StorageStatsResponse,
  Rename
} from 'types'

/**
 * Take the unencrypted file or thumbnail and encrypt it with the file key
 */
export async function encrypt(
  unencrypted: AppFileUnencryptedPart,
  publicKey: string
): Promise<AppFileEncryptedPart> {
  const key = unencrypted.key ? unencrypted.key : await cryptfns.aes.generateKey()

  const encrypted_name = await cryptfns.aes.encryptString(unencrypted.name, key)
  const encrypted_thumbnail = unencrypted.thumbnail
    ? await cryptfns.aes.encryptString(unencrypted.thumbnail, key)
    : undefined

  const keyHex = cryptfns.uint8.toHex(key)
  const encrypted_key = await cryptfns.rsa.encryptMessage(keyHex, publicKey)

  return {
    encrypted_key,
    encrypted_name,
    encrypted_thumbnail
  }
}

/**
 * Return the unencrypted file parts
 */
export async function decrypt(
  encrypted: AppFileEncryptedPart,
  privateKey: string
): Promise<AppFileUnencryptedPart> {
  const keyHex = await cryptfns.rsa.decryptMessage(privateKey, encrypted.encrypted_key)
  const key = cryptfns.uint8.fromHex(keyHex)

  const name = await cryptfns.aes.decryptString(encrypted.encrypted_name, key)
  const thumbnail = encrypted.encrypted_thumbnail
    ? await cryptfns.aes.decryptString(encrypted.encrypted_thumbnail, key)
    : undefined

  return {
    key,
    name,
    thumbnail
  }
}

/**
 * Create a file or directory on the server
 */
export async function create(keypair: KeyPair, unencrypted: CreateFile): Promise<AppFile> {
  if (!keypair.publicKey) {
    throw new Error('Cannot create file without public key')
  }

  if (!keypair.input) {
    throw new Error('Cannot create file without private key')
  }

  const encryptedParts = await encrypt(unencrypted, keypair.publicKey)

  const createFile: EncryptedCreateFile = {
    search_tokens_hashed: unencrypted.search_tokens_hashed,
    name_hash: cryptfns.sha256.digest(unencrypted.name),
    mime: unencrypted.mime,
    size: unencrypted.size,
    chunks: unencrypted.chunks,
    file_id: unencrypted.file_id,
    file_modified_at: unencrypted.file_modified_at,
    ...encryptedParts
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
  const unencryptedPart = await decrypt(file, keypair.input)

  return {
    ...file,
    ...unencryptedPart
  }
}

/**
 * Rename a file or a directory
 */
export async function rename(
  keypair: KeyPair,
  file: AppFile,
  unencrypted: Rename
): Promise<AppFile> {
  if (!keypair.publicKey) {
    throw new Error('Cannot rename file without public key')
  }

  if (!keypair.input) {
    throw new Error('Cannot rename file without private key')
  }

  const encryptedParts = await encrypt({ key: file.key, name: unencrypted.name }, keypair.publicKey)

  const rename: EncryptedRename = {
    search_tokens_hashed: unencrypted.search_tokens_hashed,
    name_hash: cryptfns.sha256.digest(unencrypted.name),
    encrypted_name: encryptedParts.encrypted_name
  }

  const response = await Api.put<EncryptedRename, AppFile>(
    `/api/storage/${file.id}`,
    undefined,
    rename
  )

  if (!response?.body?.id) {
    throw new Error('Failed to create file')
  }

  const renamedFile = response.body
  const unencryptedPart = await decrypt(renamedFile, keypair.input)

  return {
    ...renamedFile,
    ...unencryptedPart
  }
}

/**
 * Get file or directory metadata
 */
export async function get(keypair: KeyPair, file_id: string): Promise<AppFile> {
  if (!keypair.input) {
    throw new Error('Cannot get file without private key')
  }

  const response = await Api.get<AppFile>(`/api/storage/${file_id}/metadata`, undefined)

  if (!response?.body?.id) {
    throw new Error('Failed to get file or directory')
  }

  const file = response.body
  const unencryptedPart = await decrypt(file, keypair.input)

  return { ...file, ...unencryptedPart }
}

/**
 *  Lookup directory or file by its name and parent directory
 */
export async function getByName(
  keypair: KeyPair,
  name: string,
  parent_id?: string
): Promise<AppFile> {
  if (!keypair.input) {
    throw new Error('Cannot get file without private key')
  }

  const nameHash = cryptfns.sha256.digest(name)

  if (parent_id !== undefined || typeof parent_id !== 'number') {
    parent_id = undefined
  }

  const response = await Api.get<AppFile>(`/api/storage/${nameHash}/name-hash`, { parent_id })

  if (!response?.body?.id) {
    throw new Error('Failed to get file or directory')
  }

  const file = response.body
  const unencryptedPart = await decrypt(file, keypair.input)

  return { ...file, ...unencryptedPart }
}

/**
 * Get file or directory metadata
 */
export async function find(parameters: Parameters): Promise<FileResponse> {
  // @ts-ignore
  if (typeof parameters.dir_id !== 'undefined' && typeof parameters.dir_id !== 'string') {
    delete parameters.dir_id
  }

  const response = await Api.get<FileResponse>(`/api/storage`, parameters)

  return response.body || { children: [], parents: [] }
}

/**
 * Get users storage stats
 */
export async function stats(): Promise<StorageStatsResponse> {
  const response = await Api.post<undefined, StorageStatsResponse>(`/api/storage/stats`)

  return response.body || { stats: [], used_space: 0, quota: undefined }
}

/**
 * Get file or directory metadata
 */
export async function search(input: string, dir_id?: string): Promise<EncryptedAppFile[]> {
  const search_tokens_hashed = cryptfns.stringToHashedTokens(input.toLowerCase())

  if (!search_tokens_hashed.length) {
    return []
  }

  const body = {
    search_tokens_hashed,
    dir_id,
    limit: 10,
    skip: 0
  }

  const response = await Api.post<SearchQuery, EncryptedAppFile[]>(
    `/api/storage/search`,
    undefined,
    body
  )

  return response.body || []
}

/**
 * Get file or directory metadata
 */
export async function remove(fileId: string): Promise<void> {
  await Api.delete(`/api/storage/${fileId}`)
}
