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
  Rename,
  EncryptedRename,
  DeleteManyFiles,
  MoveManyFiles,
  TransferTokenResponse
} from 'types'

/**
 * Take the unencrypted file or thumbnail and encrypt it with the file key.
 *
 * @param cipher  Cipher identifier (default: `cryptfns.cipher.DEFAULT_CIPHER`).
 *                Controls which algorithm is used for the file key generation,
 *                name encryption, thumbnail encryption, and chunk encryption.
 */
export async function encrypt(
  unencrypted: AppFileUnencryptedPart,
  publicKey: string,
  cipher = cryptfns.cipher.DEFAULT_CIPHER
): Promise<AppFileEncryptedPart> {
  const key = unencrypted.key ? unencrypted.key : await cryptfns.cipher.generateKey(cipher)

  const encrypted_name = await cryptfns.cipher.encryptString(cipher, unencrypted.name, key)
  const encrypted_thumbnail = unencrypted.thumbnail
    ? await cryptfns.cipher.encryptString(cipher, unencrypted.thumbnail, key)
    : undefined

  const keyHex = cryptfns.uint8.toHex(key)
  const encrypted_key = await cryptfns.rsa.encryptMessage(keyHex, publicKey)

  return {
    encrypted_key,
    encrypted_name,
    encrypted_thumbnail,
    cipher
  }
}

/**
 * Return the unencrypted file parts.
 *
 * Reads `encrypted.cipher` to determine which cipher was used when the file was created.
 * Falls back to `"ascon128a"` for existing files that predate the cipher field.
 */
export async function decrypt(
  encrypted: AppFileEncryptedPart,
  privateKey: string
): Promise<AppFileUnencryptedPart> {
  const cipher = encrypted.cipher

  const keyHex = await cryptfns.rsa.decryptMessage(privateKey, encrypted.encrypted_key)
  const key = cryptfns.uint8.fromHex(keyHex)

  const name = await cryptfns.cipher.decryptString(cipher, encrypted.encrypted_name, key)
  const thumbnail = encrypted.encrypted_thumbnail
    ? await cryptfns.cipher.decryptString(cipher, encrypted.encrypted_thumbnail, key)
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

  const encryptedParts = await encrypt(unencrypted, keypair.publicKey, unencrypted.cipher)

  const createFile: EncryptedCreateFile = {
    search_tokens_hashed: unencrypted.search_tokens_hashed,
    name_hash: cryptfns.sha256.digest(unencrypted.name),
    mime: unencrypted.mime,
    size: unencrypted.size,
    chunks: unencrypted.chunks,
    file_id: unencrypted.file_id,
    file_modified_at: unencrypted.file_modified_at,
    md5: unencrypted.md5,
    sha1: unencrypted.sha1,
    sha256: unencrypted.sha256,
    blake2b: unencrypted.blake2b,
    editable: unencrypted.editable,
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

  if (parent_id !== undefined && typeof parent_id !== 'string') {
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
export async function search(
  input: string,
  options?: { dir_id?: string; editable?: boolean; limit?: number }
): Promise<EncryptedAppFile[]> {
  const body = {
    search: input,
    dir_id: options?.dir_id,
    editable: options?.editable,
    limit: options?.limit ?? 10,
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
 * Persist file content hashes (sha256) to the server.
 * Uses an upload transfer token so the request succeeds even after the session expires.
 * Returns the updated AppFile record.
 */
export async function updateHashes(fileId: string, sha256: string): Promise<AppFile> {
  const { token } = await requestTransferToken(fileId, 'upload')
  const api = new Api({ ...new Api().toJson(), jwtToken: token, refreshToken: undefined })

  const response = await api.make<{ sha256: string }, AppFile>(
    'put',
    `/api/storage/${fileId}/hashes`,
    undefined,
    { sha256 }
  )

  if (!response?.body?.id) {
    throw new Error('Failed to update file hashes')
  }

  return response.body
}

/**
 * Toggle the `editable` flag on an existing file.
 * Used to convert a regular file into an editable note (or back).
 */
export async function setEditable(
  keypair: KeyPair,
  fileId: string,
  editable: boolean
): Promise<AppFile> {
  if (!keypair.input) {
    throw new Error('Cannot update file without private key')
  }

  const response = await Api.put<{ editable: boolean }, AppFile>(
    `/api/storage/${fileId}/editable`,
    undefined,
    { editable }
  )

  if (!response?.body?.id) {
    throw new Error('Failed to update editable flag')
  }

  const file = response.body
  const unencryptedPart = await decrypt(file, keypair.input)

  return { ...file, ...unencryptedPart }
}

/**
 * Get file or directory metadata
 */
export async function remove(fileId: string): Promise<void> {
  await Api.delete(`/api/storage/${fileId}`)
}

/**
 * Remove many files and folders at once
 */
export async function removeAll(body: DeleteManyFiles): Promise<void> {
  await Api.post<DeleteManyFiles, undefined>(`/api/storage/delete-many`, undefined, body)
}

/**
 * Remove many files and folders at once
 */
export async function moveMany(body: MoveManyFiles): Promise<void> {
  await Api.post<MoveManyFiles, undefined>(`/api/storage/move-many`, undefined, body)
}

/**
 * Request a long-lived transfer token scoped to a single file and action.
 * The token is a JWT valid for `long_term_session_duration_days` (default 30 days)
 * and can only be used for the specified action on the specified file.
 */
export async function requestTransferToken(
  fileId: string,
  action: 'upload' | 'download'
): Promise<TransferTokenResponse> {
  const response = await Api.post<{ file_id: string; action: string }, TransferTokenResponse>(
    '/api/auth/transfer-token',
    undefined,
    { file_id: fileId, action }
  )

  if (!response?.body?.token) {
    throw new Error('Failed to request transfer token')
  }

  return response.body
}
