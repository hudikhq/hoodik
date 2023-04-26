import Api from '../api'
import { FileMetadata } from './metadata'
import * as cryptfns from '../cryptfns'

import type { KeyPair } from '../cryptfns/rsa'
import type { AppFile, CreateFile, EncryptedCreateFile, FileResponse, Parameters } from '../types'

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

  const response = await Api.get<FileResponse>(`/api/storage`, parameters)

  return response.body || { children: [], parents: [] }
}

/**
 * Get file or directory metadata
 */
export async function remove(fileId: number): Promise<void> {
  await Api.delete(`/api/storage/${fileId}`)
}
