import Api from '../api'
import * as cryptfns from '../cryptfns'
import { CHUNK_SIZE_BYTES } from '../constants'
import { uploadChunk } from './upload/sync'
import * as meta from './meta'

import type { AppFile, KeyPair, CreateFile } from 'types'

export interface ReplaceContentRequest {
  size: number
  chunks: number
  cipher?: string
  encrypted_name?: string
  encrypted_thumbnail?: string
  search_tokens_hashed?: string[]
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
 * Save new content to an existing editable file.
 * Replaces all chunks and re-uploads.
 */
export async function saveFileContent(
  file: AppFile,
  content: string
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

  const updatedFile = await replaceContent(file.id, {
    size,
    chunks: chunkCount,
    search_tokens_hashed: searchTokens
  })

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
