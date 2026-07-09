import Api, { ErrorResponse } from '../api'
import * as cryptfns from '../cryptfns'
import { CHUNK_SIZE_BYTES } from '../constants'
import { uploadChunk } from './upload/sync'
import * as meta from './meta'
import {
  uploadIntoSharedFolder,
  type UploadIntoSharedFolderArgs
} from '../shares/editable'
import { trustedFingerprintsStore } from '../shares'
import { uuidv4, utcStringFromLocal } from '..'

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
 * Whether a new file/note created at this parent must take the multi-key
 * shared-folder path instead of the single-owner `POST /api/storage` one.
 * The backend's owner-only check on the regular create rejects every
 * non-owner parent ("parent_directory_not_found"), so any folder the
 * caller doesn't own must route through `uploadIntoSharedFolder` to fan
 * the new file's key out to all current members. Owned folders that have
 * been shared go through the same path so the cascade fires without the
 * owner having to re-upload manually.
 */
export function needsMultikeyCreate(parent: AppFile | null | undefined): boolean {
  if (!parent) return false
  if (parent.mime !== 'dir') return false
  if (parent.is_owner === false) return true
  return parent.members_signed_at != null
}

/**
 * Create a new markdown note with initial heading content,
 * upload it as a single chunk, and return the created file.
 *
 * When `parent` is supplied as an `AppFile` and it's a shared folder, the
 * create routes through the multi-key pipeline so every current member
 * receives an RSA-wrapped copy of the file key. Callers that only have
 * the parent id can pass a string and the helper will fetch the
 * `AppFile` lazily — the round trip is the cost of not threading the
 * full row through every caller.
 */
export async function createNote(
  keypair: KeyPair,
  name: string,
  parent?: AppFile | string | null,
  callerUserId?: string
): Promise<AppFile> {
  const fileName = name.endsWith('.md') ? name : `${name}.md`
  const tokens = cryptfns.stringToHashedTokens(fileName.toLowerCase())

  const initialContent = `# ${fileName.replace(/\.md$/i, '')}\n`
  const contentBytes = new TextEncoder().encode(initialContent)

  let parentFile: AppFile | null = null
  if (parent && typeof parent === 'object') {
    parentFile = parent
  } else if (typeof parent === 'string' && parent) {
    try {
      parentFile = await meta.get(keypair, parent)
    } catch {
      // Caller-passed id might predate a sync — fall through to the
      // regular create path; the server will produce a clearer error if
      // the row truly does not exist.
      parentFile = null
    }
  }

  if (needsMultikeyCreate(parentFile)) {
    if (!parentFile) throw new Error('Cannot create file without parent context')
    if (!keypair.input || !keypair.publicKey) {
      throw new Error('Cannot create file without an active keypair')
    }
    if (!callerUserId) {
      throw new Error('Cannot create file in a shared folder without caller id')
    }
    return createNoteInSharedFolder({
      keypair,
      callerUserId,
      parent: parentFile,
      fileName,
      contentBytes,
      tokens
    })
  }

  const createData: CreateFile = {
    name: fileName,
    mime: 'text/markdown',
    editable: true,
    size: contentBytes.length,
    chunks: 1,
    search_tokens_hashed: tokens,
    file_id: parentFile?.id ?? (typeof parent === 'string' ? parent : undefined),
    cipher: cryptfns.cipher.defaultCipher()
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

async function createNoteInSharedFolder(args: {
  keypair: KeyPair
  callerUserId: string
  parent: AppFile
  fileName: string
  contentBytes: Uint8Array
  tokens: string[]
}): Promise<AppFile> {
  const cipher = cryptfns.cipher.defaultCipher()
  const fileKey = await cryptfns.cipher.generateKey(cipher)
  const fileKeyHex = cryptfns.uint8.toHex(fileKey)
  const encryptedName = await cryptfns.cipher.encryptString(cipher, args.fileName, fileKey)
  const nameHash = cryptfns.sha256.digest(args.fileName)
  const newFileId = uuidv4()
  const modified = new Date()

  const uploadArgs: UploadIntoSharedFolderArgs = {
    callerUserId: args.callerUserId,
    callerPrivateKey: args.keypair.input as string,
    callerPublicKey: args.keypair.publicKey as string,
    payload: {
      newFileId,
      parentFileId: args.parent.id,
      fileKeyHex,
      nameHash,
      encryptedName,
      mime: 'text/markdown',
      size: args.contentBytes.length,
      chunks: 1,
      cipher,
      editable: true,
      fileModifiedAt: utcStringFromLocal(modified),
      searchTokensHashed: args.tokens
    },
    trustedFingerprints: trustedFingerprintsStore(),
    onUnknownMember: async () => true
  }

  await uploadIntoSharedFolder(uploadArgs)

  const { token } = await meta.requestTransferToken(newFileId, 'upload')
  const api = new Api({ ...new Api().toJson(), jwtToken: token, refreshToken: undefined })

  const placeholder = {
    id: newFileId,
    user_id: args.callerUserId,
    is_owner: true,
    name_hash: nameHash,
    mime: 'text/markdown',
    size: args.contentBytes.length,
    chunks: 1,
    file_id: args.parent.id,
    file_modified_at: Math.floor(modified.getTime() / 1000),
    created_at: Math.floor(Date.now() / 1000),
    is_new: true,
    editable: true,
    active_version: 1,
    encrypted_key: '',
    encrypted_name: encryptedName,
    cipher,
    key: fileKey,
    name: args.fileName,
    temporaryId: newFileId
  } as AppFile & { temporaryId: string }

  await uploadChunk(
    {
      ...placeholder,
      file: new File([args.contentBytes], args.fileName)
    },
    args.contentBytes,
    0,
    0,
    api
  )

  return await meta.get(args.keypair, newFileId)
}
