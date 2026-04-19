import { describe, it, expect, vi, beforeEach } from 'vitest'

/**
 * save.ts exercises the happy path (replaceContent → transfer-token →
 * chunked upload) and two error paths (409 → SaveConflictError, any
 * other failure → bubbles up). We mock the IO surface — Api, meta,
 * cryptfns, uploadChunk — so the tests run as pure logic without a
 * network or a WASM runtime.
 *
 * Vitest 0.30 doesn't have `vi.hoisted`, so the factories below create
 * their own `vi.fn()`s and we retrieve them via dynamic import afterwards.
 */

vi.mock('../services/api', () => {
  class ErrorResponse extends Error {
    status: number
    body: unknown
    constructor(status: number, body?: unknown) {
      super(`status ${status}`)
      this.status = status
      this.body = body
      this.name = 'ErrorResponse'
    }
  }
  const put = vi.fn()
  return {
    default: class {
      constructor(_?: unknown) {}
      toJson() {
        return {}
      }
      static put = put
    },
    ErrorResponse
  }
})

vi.mock('../services/storage/meta', () => ({
  requestTransferToken: vi.fn(() => Promise.resolve({ token: 'transfer-token' })),
  create: vi.fn((_kp: unknown, input: { name: string; mime: string; editable?: boolean }) =>
    Promise.resolve({
      id: 'created-file-id',
      name: input.name,
      mime: input.mime,
      editable: !!input.editable,
      chunks: 1,
      size: 42,
      key: new Uint8Array(32)
    })
  )
}))

vi.mock('../services/cryptfns', () => ({
  stringToHashedTokens: vi.fn(() => ['tok-a', 'tok-b']),
  sha256: { digest: vi.fn(() => 'name-hash') },
  cipher: { DEFAULT_CIPHER: 'aegis-128l' }
}))

vi.mock('../services/storage/upload/sync', () => ({
  uploadChunk: vi.fn(() => Promise.resolve(undefined))
}))

vi.mock('../services/constants', () => ({
  CHUNK_SIZE_BYTES: 1_048_576
}))

import { replaceContent, saveFileContent, createNote, SaveConflictError } from '../services/storage/save'
import Api, { ErrorResponse } from '../services/api'
import { requestTransferToken, create as metaCreate } from '../services/storage/meta'
import { uploadChunk } from '../services/storage/upload/sync'
import { stringToHashedTokens } from '../services/cryptfns'
import type { AppFile, KeyPair } from 'types'

const ApiPutMock = (Api as unknown as { put: ReturnType<typeof vi.fn> }).put

function makeAppFile(partial: Partial<AppFile> = {}): AppFile {
  return {
    id: 'file-1',
    user_id: 'u1',
    is_owner: true,
    name_hash: 'h',
    name: 'note.md',
    mime: 'text/markdown',
    size: 10,
    chunks: 1,
    encrypted_key: '',
    encrypted_name: '',
    cipher: 'aegis-128l',
    editable: true,
    active_version: 1,
    file_modified_at: 0,
    created_at: 0,
    is_new: false,
    key: new Uint8Array(32),
    ...partial
  } as AppFile
}

describe('replaceContent', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('UNIT: PUTs /api/storage/{id}/content and returns the updated file', async () => {
    const updated = makeAppFile({ id: 'file-1', active_version: 2 })
    ApiPutMock.mockResolvedValueOnce({ body: updated })

    const result = await replaceContent('file-1', {
      size: 10,
      chunks: 1,
      search_tokens_hashed: ['a']
    })

    expect(ApiPutMock).toHaveBeenCalledTimes(1)
    expect(ApiPutMock).toHaveBeenCalledWith(
      '/api/storage/file-1/content',
      undefined,
      { size: 10, chunks: 1, search_tokens_hashed: ['a'] }
    )
    expect(result.id).toBe('file-1')
    expect(result.active_version).toBe(2)
  })

  it('UNIT: throws when the server returns an empty body', async () => {
    ApiPutMock.mockResolvedValueOnce({ body: undefined })
    await expect(replaceContent('file-1', { size: 1, chunks: 1 })).rejects.toThrow(
      'Failed to replace file content'
    )
  })

  it('UNIT: forwards the `force` flag verbatim', async () => {
    ApiPutMock.mockResolvedValueOnce({ body: makeAppFile() })
    await replaceContent('file-1', { size: 1, chunks: 1, force: true })
    expect(ApiPutMock).toHaveBeenCalledWith(
      '/api/storage/file-1/content',
      undefined,
      expect.objectContaining({ force: true })
    )
  })
})

describe('saveFileContent', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('UNIT: rejects without a file key', async () => {
    const file = makeAppFile({ key: undefined })
    await expect(saveFileContent(file, 'hello')).rejects.toThrow('File key is required')
  })

  it('UNIT: orchestrates replaceContent → transfer-token → uploadChunk', async () => {
    const file = makeAppFile()
    const updated = makeAppFile({ active_version: 2 })
    ApiPutMock.mockResolvedValueOnce({ body: updated })

    const out = await saveFileContent(file, '# hello world')

    expect(ApiPutMock).toHaveBeenCalledTimes(1)
    expect(requestTransferToken).toHaveBeenCalledWith(file.id, 'upload')
    expect(uploadChunk).toHaveBeenCalledTimes(1)
    // saveFileContent preserves the unencrypted metadata from the caller's
    // file (key, name, thumbnail) on the returned AppFile.
    expect(out.key).toBe(file.key)
    expect(out.name).toBe(file.name)
    expect(out.active_version).toBe(2)
  })

  it('UNIT: pads empty content with a space so backend size >= 1', async () => {
    const file = makeAppFile()
    ApiPutMock.mockResolvedValueOnce({ body: makeAppFile() })

    await saveFileContent(file, '')

    const [, , body] = ApiPutMock.mock.calls[0]
    expect(body.size).toBe(1)
    expect(body.chunks).toBe(1)
  })

  it('UNIT: computes chunk count for multi-chunk content', async () => {
    const file = makeAppFile()
    ApiPutMock.mockResolvedValueOnce({ body: makeAppFile() })

    // CHUNK_SIZE_BYTES = 1_048_576 (mock). 3 MiB of ASCII ends up in 3 chunks.
    const big = 'x'.repeat(1_048_576 * 3)
    await saveFileContent(file, big)

    const [, , body] = ApiPutMock.mock.calls[0]
    expect(body.chunks).toBe(3)
    expect(body.size).toBe(1_048_576 * 3)
    expect(uploadChunk).toHaveBeenCalledTimes(3)
  })

  it('UNIT: passes search tokens from cryptfns into the body', async () => {
    const file = makeAppFile()
    ApiPutMock.mockResolvedValueOnce({ body: makeAppFile() })

    await saveFileContent(file, 'hello')

    expect(stringToHashedTokens).toHaveBeenCalledWith('hello')
    const [, , body] = ApiPutMock.mock.calls[0]
    expect(body.search_tokens_hashed).toEqual(['tok-a', 'tok-b'])
  })

  it('UNIT: 409 from the server becomes a SaveConflictError carrying the draft content', async () => {
    const file = makeAppFile()
    ApiPutMock.mockRejectedValueOnce(new ErrorResponse(409))

    try {
      await saveFileContent(file, 'my draft content')
      throw new Error('expected a SaveConflictError')
    } catch (err) {
      expect(err).toBeInstanceOf(SaveConflictError)
      const conflict = err as SaveConflictError
      expect(conflict.fileId).toBe(file.id)
      expect(conflict.originalContent).toBe('my draft content')
    }

    // A 409 means nothing was written: neither transfer-token nor upload
    // should have been requested.
    expect(requestTransferToken).not.toHaveBeenCalled()
    expect(uploadChunk).not.toHaveBeenCalled()
  })

  it('UNIT: non-409 ErrorResponse is re-thrown verbatim (not wrapped)', async () => {
    const file = makeAppFile()
    const err = new ErrorResponse(500)
    ApiPutMock.mockRejectedValueOnce(err)

    await expect(saveFileContent(file, 'content')).rejects.toBe(err)
  })

  it('UNIT: forwards force=true to replaceContent', async () => {
    const file = makeAppFile()
    ApiPutMock.mockResolvedValueOnce({ body: makeAppFile() })

    await saveFileContent(file, 'content', true)

    const [, , body] = ApiPutMock.mock.calls[0]
    expect(body.force).toBe(true)
  })

  it('UNIT: defaults force to false when omitted', async () => {
    const file = makeAppFile()
    ApiPutMock.mockResolvedValueOnce({ body: makeAppFile() })

    await saveFileContent(file, 'content')

    const [, , body] = ApiPutMock.mock.calls[0]
    expect(body.force).toBe(false)
  })
})

describe('createNote', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  function makeKeypair(): KeyPair {
    return {
      publicKey: 'pub',
      input: 'priv',
      fingerprint: 'fp'
    } as KeyPair
  }

  it('UNIT: appends .md when missing and defaults folderId to undefined', async () => {
    await createNote(makeKeypair(), 'shopping-list')

    expect(metaCreate).toHaveBeenCalledTimes(1)
    const [, data] = (metaCreate as unknown as { mock: { calls: unknown[][] } }).mock.calls[0] as [
      unknown,
      { name: string; mime: string; editable: boolean; chunks: number; file_id?: string }
    ]
    expect(data.name).toBe('shopping-list.md')
    expect(data.mime).toBe('text/markdown')
    expect(data.editable).toBe(true)
    expect(data.chunks).toBe(1)
    expect(data.file_id).toBeUndefined()
  })

  it('UNIT: keeps an existing .md suffix without doubling it', async () => {
    await createNote(makeKeypair(), 'notes.md')
    const [, data] = (metaCreate as unknown as { mock: { calls: unknown[][] } }).mock.calls[0] as [
      unknown,
      { name: string }
    ]
    expect(data.name).toBe('notes.md')
  })

  it('UNIT: uploads the initial chunk with the created file metadata', async () => {
    await createNote(makeKeypair(), 'hello')

    expect(requestTransferToken).toHaveBeenCalledWith('created-file-id', 'upload')
    expect(uploadChunk).toHaveBeenCalledTimes(1)
    const [uploadFile, chunkBytes, chunkIndex] = (
      uploadChunk as unknown as { mock: { calls: unknown[][] } }
    ).mock.calls[0] as [{ temporaryId: string; file: File }, Uint8Array, number]
    expect(uploadFile.temporaryId).toBe('created-file-id')
    expect(uploadFile.file).toBeInstanceOf(File)
    expect(chunkIndex).toBe(0)
    expect(chunkBytes.length).toBeGreaterThan(0)
  })

  it('UNIT: passes folderId through when provided', async () => {
    await createNote(makeKeypair(), 'note', 'folder-uuid')
    const [, data] = (metaCreate as unknown as { mock: { calls: unknown[][] } }).mock.calls[0] as [
      unknown,
      { file_id?: string }
    ]
    expect(data.file_id).toBe('folder-uuid')
  })

  it('UNIT: initial content is a heading derived from the note name', async () => {
    await createNote(makeKeypair(), 'shopping-list')
    const [, chunkBytes] = (uploadChunk as unknown as { mock: { calls: unknown[][] } }).mock
      .calls[0] as [unknown, Uint8Array]
    const decoded = new TextDecoder().decode(chunkBytes)
    expect(decoded).toBe('# shopping-list\n')
  })
})
