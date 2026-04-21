import { describe, it, expect, vi, beforeEach } from 'vitest'

/**
 * versions.ts is a thin REST client over
 *   /api/storage/{file_id}/versions[/{version}[/restore|/fork]]
 * It has no client-side crypto — every byte it sends or receives is
 * already encrypted. Tests assert the right HTTP verb, the right URL,
 * and that the returned body is passed through unchanged.
 *
 * The helpers use Api's static methods (Api.get/post/delete) and an
 * instance method (new Api().download). Both paths are mocked so the
 * tests can run in jsdom without real fetch.
 */

vi.mock('../services/api', () => {
  const get = vi.fn()
  const post = vi.fn()
  const del = vi.fn()
  const download = vi.fn()
  class Api {
    static get = get
    static post = post
    static delete = del
    // Prototype-level so tests can swap the implementation before calling
    // the function under test (`new Api().download(...)` looks it up here).
    download: (...args: unknown[]) => unknown
    constructor() {
      this.download = download
    }
  }
  return { default: Api }
})

import { list, restore, fork, remove, purgeAll, downloadChunk } from '../services/storage/versions'
import Api from '../services/api'
import type { AppFile, FileVersion } from 'types'

type Mocked<T> = T & { mock: { calls: unknown[][] } }
const ApiGet = (Api as unknown as { get: Mocked<(...args: unknown[]) => unknown> }).get
const ApiPost = (Api as unknown as { post: Mocked<(...args: unknown[]) => unknown> }).post
const ApiDelete = (Api as unknown as { delete: Mocked<(...args: unknown[]) => unknown> }).delete

function makeFile(overrides: Partial<AppFile> = {}): AppFile {
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
    active_version: 2,
    file_modified_at: 0,
    created_at: 0,
    is_new: false,
    ...overrides
  } as AppFile
}

function makeVersion(overrides: Partial<FileVersion> = {}): FileVersion {
  return {
    id: 'v1',
    file_id: 'file-1',
    version: 1,
    user_id: 'u1',
    is_anonymous: false,
    size: 10,
    chunks: 1,
    sha256: 'abc',
    created_at: 0,
    ...overrides
  }
}

describe('list', () => {
  beforeEach(() => vi.clearAllMocks())

  it('UNIT: GETs /api/storage/{id}/versions and returns the list', async () => {
    const versions = [makeVersion({ version: 2 }), makeVersion({ version: 1 })]
    ;(ApiGet as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce({ body: versions })

    const result = await list('file-1')

    expect(ApiGet).toHaveBeenCalledWith('/api/storage/file-1/versions')
    expect(result).toEqual(versions)
  })

  it('UNIT: returns [] when the server responds with no body', async () => {
    ;(ApiGet as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce(undefined)
    const result = await list('file-1')
    expect(result).toEqual([])
  })

  it('UNIT: returns [] when body is missing', async () => {
    ;(ApiGet as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce({ body: undefined })
    const result = await list('file-1')
    expect(result).toEqual([])
  })
})

describe('restore', () => {
  beforeEach(() => vi.clearAllMocks())

  it('UNIT: POSTs /versions/{v}/restore and returns the flipped file', async () => {
    const updated = makeFile({ active_version: 3 })
    ;(ApiPost as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce({ body: updated })

    const result = await restore('file-1', 2)

    expect(ApiPost).toHaveBeenCalledWith('/api/storage/file-1/versions/2/restore')
    expect(result.active_version).toBe(3)
  })

  it('UNIT: throws when the server returns no body', async () => {
    ;(ApiPost as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce({ body: undefined })
    await expect(restore('file-1', 2)).rejects.toThrow('Failed to restore v2')
  })

  it('UNIT: throws when body has no id (malformed response)', async () => {
    ;(ApiPost as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce({ body: { foo: 1 } })
    await expect(restore('file-1', 2)).rejects.toThrow('Failed to restore v2')
  })
})

describe('fork', () => {
  beforeEach(() => vi.clearAllMocks())

  it('UNIT: POSTs /versions/{v}/fork with the encrypted payload', async () => {
    const created = makeFile({ id: 'forked-id', name: 'forked.md' })
    ;(ApiPost as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce({ body: created })

    const body = {
      name_hash: 'h',
      encrypted_name: 'en',
      encrypted_key: 'ek',
      mime: 'text/markdown',
      cipher: 'aegis-128l',
      editable: true
    }
    const result = await fork('file-1', 2, body)

    expect(ApiPost).toHaveBeenCalledWith(
      '/api/storage/file-1/versions/2/fork',
      undefined,
      body
    )
    expect(result.id).toBe('forked-id')
  })

  it('UNIT: rejects when the server returns no body', async () => {
    ;(ApiPost as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce({ body: undefined })
    await expect(
      fork('file-1', 1, {
        name_hash: '',
        encrypted_name: '',
        encrypted_key: '',
        mime: 'text/markdown',
        cipher: 'aegis-128l'
      })
    ).rejects.toThrow('Failed to fork v1')
  })

  it('UNIT: search_tokens_hashed flows through untouched', async () => {
    ;(ApiPost as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce({ body: makeFile() })
    const body = {
      name_hash: 'h',
      encrypted_name: 'en',
      encrypted_key: 'ek',
      mime: 'text/markdown',
      cipher: 'aegis-128l',
      search_tokens_hashed: ['t1', 't2', 't3']
    }
    await fork('file-1', 2, body)
    expect(ApiPost).toHaveBeenCalledWith(
      '/api/storage/file-1/versions/2/fork',
      undefined,
      expect.objectContaining({ search_tokens_hashed: ['t1', 't2', 't3'] })
    )
  })
})

describe('remove', () => {
  beforeEach(() => vi.clearAllMocks())

  it('UNIT: DELETEs /versions/{v}', async () => {
    ;(ApiDelete as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce(undefined)
    await remove('file-1', 3)
    expect(ApiDelete).toHaveBeenCalledWith('/api/storage/file-1/versions/3')
  })
})

describe('purgeAll', () => {
  beforeEach(() => vi.clearAllMocks())

  it('UNIT: DELETEs /versions (plural, no version segment)', async () => {
    ;(ApiDelete as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce(undefined)
    await purgeAll('file-1')
    expect(ApiDelete).toHaveBeenCalledWith('/api/storage/file-1/versions')
  })

  it('UNIT: does not accidentally include a version segment', async () => {
    ;(ApiDelete as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce(undefined)
    await purgeAll('file-1')
    const [url] = (ApiDelete as unknown as { mock: { calls: unknown[][] } }).mock.calls[0] as [string]
    expect(url).not.toMatch(/\/versions\/\d+$/)
  })
})

describe('downloadChunk', () => {
  // The `download` handle is shared across every `new Api()` instance in
  // the vi.mock factory. Grab it by peeking at a fresh instance once.
  const ApiDownload = (new (Api as unknown as new () => { download: ReturnType<typeof vi.fn> })())
    .download

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('UNIT: streams chunk bytes via Api.download and concatenates them', async () => {
    // Simulate a two-read stream: "01 02 03" then "04 05" then done.
    let step = 0
    const reader = {
      read: vi.fn().mockImplementation(() => {
        step += 1
        if (step === 1) return Promise.resolve({ value: new Uint8Array([1, 2, 3]), done: false })
        if (step === 2) return Promise.resolve({ value: new Uint8Array([4, 5]), done: false })
        return Promise.resolve({ value: undefined, done: true })
      })
    }
    const body = { getReader: () => reader } as unknown as ReadableStream<Uint8Array>
    ApiDownload.mockResolvedValueOnce({ body })

    const result = await downloadChunk('file-1', 2, 0)

    expect(result).toEqual(new Uint8Array([1, 2, 3, 4, 5]))
    expect(ApiDownload).toHaveBeenCalled()
    const [url] = ApiDownload.mock.calls[0] as [string]
    expect(url).toBe('/api/storage/file-1/versions/2?chunk=0')
  })

  it('UNIT: throws when the response has no body', async () => {
    ApiDownload.mockResolvedValueOnce({ body: null })
    await expect(downloadChunk('file-1', 2, 0)).rejects.toThrow('Failed to download chunk 0 of v2')
  })

  it('UNIT: forwards the AbortSignal to Api.download', async () => {
    const controller = new AbortController()
    const reader = {
      read: vi.fn().mockResolvedValue({ value: undefined, done: true })
    }
    ApiDownload.mockResolvedValueOnce({ body: { getReader: () => reader } })

    await downloadChunk('file-1', 3, 1, controller.signal)

    // Api.download signature: (path, query, body, signal)
    const args = ApiDownload.mock.calls[0]
    expect(args[3]).toBe(controller.signal)
  })
})
