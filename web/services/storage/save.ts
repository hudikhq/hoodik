import Api, { ErrorResponse } from '../api'
import * as cryptfns from '../cryptfns'
import { CHUNK_SIZE_BYTES } from '../constants'
import { uploadChunk } from './upload/sync'
import * as meta from './meta'

import type { AppFile, KeyPair, CreateFile } from 'types'

export interface ReplaceContentRequest {
  size: number
  chunks: number
  encrypted_name?: string
  encrypted_thumbnail?: string
  search_tokens_hashed?: string[]
  /**
   * Abandon any in-flight pending edit on the server and start fresh.
   * Set when the previous save died mid-way (the user picks "discard"
   * on the conflict prompt). Without this flag, a second save while
   * pending exists returns 409.
   */
  force?: boolean
}

/**
 * Thrown by [[saveFileContent]] when the server returns 409 — another
 * edit is in progress. The caller can re-issue the save with `force =
 * true` to abandon the previous pending and overwrite.
 */
export class SaveConflictError extends Error {
  readonly fileId: string
  readonly originalContent: string

  constructor(fileId: string, originalContent: string) {
    super('another_edit_is_in_progress')
    this.name = 'SaveConflictError'
    this.fileId = fileId
    this.originalContent = originalContent
  }
}

export async function replaceContent(
  fileId: string,
  data: ReplaceContentRequest
): Promise<AppFile> {
  const response = await Api.put<ReplaceContentRequest, AppFile>(
    `/api/storage/${fileId}/content`,
    undefined,
    data
  )

  if (!response?.body?.id) {
    throw new Error('Failed to replace file content')
  }

  return response.body
}

/**
 * Save new content to an existing editable file. The default path
 * surfaces a 409 as [[SaveConflictError]] so the UI can prompt the
 * user; pass `force = true` to bypass.
 */
export async function saveFileContent(
  file: AppFile,
  content: string,
  force = false
): Promise<AppFile> {
  if (!file.key) {
    throw new Error('File key is required to save content')
  }

  // Backend requires size >= 1, so pad empty content with a space
  const safeContent = content || ' '
  const encoder = new TextEncoder()
  const contentBytes = encoder.encode(safeContent)
  const size = contentBytes.length
  const chunkCount = Math.ceil(size / CHUNK_SIZE_BYTES) || 1
  const searchTokens = cryptfns.stringToHashedTokens(safeContent)

  let updatedFile: AppFile
  try {
    updatedFile = await replaceContent(file.id, {
      size,
      chunks: chunkCount,
      search_tokens_hashed: searchTokens,
      force
    })
  } catch (err) {
    // The server's `Conflict` variant maps to HTTP 409. Repackage so
    // the UI can offer a "discard and overwrite" retry without having
    // to know about ErrorResponse internals.
    if (err instanceof ErrorResponse && err.status === 409) {
      throw new SaveConflictError(file.id, content)
    }
    throw err
  }

  const { token } = await meta.requestTransferToken(file.id, 'upload')
  const api = new Api({ ...new Api().toJson(), jwtToken: token, refreshToken: undefined })

  const uploadFile = {
    ...updatedFile,
    key: file.key,
    name: file.name,
    thumbnail: file.thumbnail,
    file: new File([contentBytes], file.name),
    temporaryId: file.id
  }

  for (let i = 0; i < chunkCount; i++) {
    const start = i * CHUNK_SIZE_BYTES
    const end = Math.min(start + CHUNK_SIZE_BYTES, size)
    const chunkData = contentBytes.slice(start, end)
    await uploadChunk(uploadFile, chunkData, i, 0, api)
  }

  return {
    ...updatedFile,
    key: file.key,
    name: file.name,
    thumbnail: file.thumbnail
  }
}

/**
 * Create a new markdown note with initial heading content,
 * upload it as a single chunk, and return the created file.
 */
export async function createNote(
  keypair: KeyPair,
  name: string,
  folderId?: string
): Promise<AppFile> {
  const fileName = name.endsWith('.md') ? name : `${name}.md`
  const tokens = cryptfns.stringToHashedTokens(fileName.toLowerCase())

  const initialContent = `# ${fileName.replace(/\.md$/i, '')}\n`
  const contentBytes = new TextEncoder().encode(initialContent)

  const createData: CreateFile = {
    name: fileName,
    mime: 'text/markdown',
    editable: true,
    size: contentBytes.length,
    chunks: 1,
    search_tokens_hashed: tokens,
    file_id: folderId,
    cipher: cryptfns.cipher.DEFAULT_CIPHER
  }

  const file = await meta.create(keypair, createData)

  const { token } = await meta.requestTransferToken(file.id, 'upload')
  const api = new Api({ ...new Api().toJson(), jwtToken: token, refreshToken: undefined })

  await uploadChunk(
    {
      ...file,
      file: new File([contentBytes], fileName),
      temporaryId: file.id
    },
    contentBytes,
    0,
    0,
    api
  )

  return file
}
