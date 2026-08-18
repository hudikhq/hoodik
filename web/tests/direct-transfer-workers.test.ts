import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

import Api from '../services/api'
import { capabilitiesStore } from '../services/shares/capabilities'
import { clearChunkUrlCache } from '../services/storage/download/direct'
import { forgetUpload } from '../services/storage/upload/direct'
import { uploadChunk } from '../services/storage/upload/sync'
import { pushUploadToWorker, startFileDownload } from '../services/storage/workers'
import * as meta from '../services/storage/meta'

import type { DownloadAppFile, UploadAppFile } from '../types'

/**
 * Both worker paths relayed every chunk for the whole life of the feature
 * while `direct-transfer.spec.ts` passed, because that spec opens a preview
 * and the preview never goes near a worker. These assert the seam itself: the
 * manifest is resolved on the main thread and reaches the worker, for both
 * directions.
 */

function manifest(urls: { chunk: number; url: string }[]) {
  return {
    body: {
      urls,
      expires_at: Math.floor(Date.now() / 1000) + 3600
    }
  }
}

function setDirectTransfer(enabled: boolean) {
  capabilitiesStore().caps = {
    sharing: { enabled: false, roles: [] },
    editable_folders: false,
    share_groups: false,
    audit_log: false,
    fork: false,
    direct_transfer: enabled
  }
}

/** Capture what the page posts to a worker, without running one. */
function stubWorker(name: 'UPLOAD' | 'DOWNLOAD' | 'HASH'): { posted: any[] } {
  const posted: any[] = []
  ;(window as any)[name] = { postMessage: (message: any) => posted.push(message) }
  return { posted }
}

const downloadFile = (): DownloadAppFile =>
  ({
    id: 'file-1',
    name: 'holiday.mov',
    chunks: 3,
    size: 12,
    cipher: 'aegis128l',
    key: new Uint8Array(32)
  }) as unknown as DownloadAppFile

const uploadFile = (): UploadAppFile =>
  ({
    id: 'file-2',
    name: 'holiday.mov',
    chunks: 2,
    cipher: 'aegis128l',
    key: new Uint8Array(32),
    file: new File([new Uint8Array(1024)], 'holiday.mov')
  }) as unknown as UploadAppFile

describe('worker transfers', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    clearChunkUrlCache()
    vi.restoreAllMocks()
    vi.spyOn(meta, 'requestTransferToken').mockResolvedValue({ token: 'transfer-token' } as never)
  })

  afterEach(() => {
    vi.restoreAllMocks()
    delete (window as any).UPLOAD
    delete (window as any).DOWNLOAD
    delete (window as any).HASH
  })

  it('hands the download worker a manifest so its chunks come from the bucket', async () => {
    setDirectTransfer(true)
    vi.spyOn(Api, 'get').mockResolvedValue(
      manifest([
        { chunk: 0, url: 'https://bucket/0' },
        { chunk: 1, url: 'https://bucket/1' },
        { chunk: 2, url: 'https://bucket/2' }
      ]) as never
    )

    const { posted } = stubWorker('DOWNLOAD')
    await startFileDownload(downloadFile())

    expect(posted[0].message.directUrls).toEqual([
      'https://bucket/0',
      'https://bucket/1',
      'https://bucket/2'
    ])
  })

  it('leaves the download worker relaying when the server has no URLs to give', async () => {
    setDirectTransfer(false)
    const get = vi.spyOn(Api, 'get')

    const { posted } = stubWorker('DOWNLOAD')
    await startFileDownload(downloadFile())

    expect(get).not.toHaveBeenCalled()
    expect(posted[0].message.directUrls).toBeUndefined()
  })

  it('declares each chunk size and hands the upload worker the URLs it gets back', async () => {
    setDirectTransfer(true)
    const post = vi
      .spyOn(Api.prototype, 'make')
      .mockResolvedValue(manifest([{ chunk: 0, url: 'https://bucket/put/0' }]) as never)

    const { posted } = stubWorker('UPLOAD')
    stubWorker('HASH')
    await pushUploadToWorker(uploadFile())

    const [method, path, , body] = post.mock.calls[0]
    expect(method).toBe('post')
    expect(path).toBe('/api/storage/file-2/upload-urls')
    // The AEAD tag on top of the payload — declaring the plaintext size would
    // have the bucket reject every chunk.
    expect(body).toEqual({ chunks: [{ chunk: 0, size: 1024 + 16 }] })

    expect(posted[0].message.directUrls).toEqual(['https://bucket/put/0'])
  })

  it('leaves the upload worker relaying when the manifest request fails', async () => {
    setDirectTransfer(true)
    vi.spyOn(Api.prototype, 'make').mockRejectedValue(new Error('direct_transfer_unavailable'))

    const { posted } = stubWorker('UPLOAD')
    stubWorker('HASH')
    await pushUploadToWorker(uploadFile())

    expect(posted[0].message.directUrls).toBeUndefined()
  })

  it('asks for no upload manifest when the server does not advertise direct transfer', async () => {
    setDirectTransfer(false)
    const make = vi.spyOn(Api.prototype, 'make')

    const { posted } = stubWorker('UPLOAD')
    stubWorker('HASH')
    await pushUploadToWorker(uploadFile())

    expect(make).not.toHaveBeenCalled()
    expect(posted[0].message.directUrls).toBeUndefined()
  })
})

/**
 * The upload path the page runs itself: note saves, forks, and the fallback
 * for when the worker is unavailable. It relayed every byte while the worker
 * path went direct, which is the same defect one layer down — so the decision
 * lives in the shared per-chunk function rather than at each of its callers.
 */
describe('page-side chunk uploads', () => {
  const noteFile = () =>
    ({
      id: 'note-1',
      name: 'notes.md',
      chunks: 2,
      size: 5 * 1024 * 1024,
      cipher: 'aegis128l',
      key: new Uint8Array(32),
      file: new File([new Uint8Array(0)], 'notes.md')
    }) as unknown as UploadAppFile

  beforeEach(() => {
    setActivePinia(createPinia())
    forgetUpload('note-1')
    vi.restoreAllMocks()
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    forgetUpload('note-1')
    vi.restoreAllMocks()
  })

  it('writes chunks into the bucket and commits once the last one lands', async () => {
    setDirectTransfer(true)

    const make = vi.spyOn(Api.prototype, 'make').mockImplementation((async (
      _method: string,
      path: string
    ) => {
      if (path.endsWith('/upload-urls')) {
        return manifest([
          { chunk: 0, url: 'https://bucket/put/0' },
          { chunk: 1, url: 'https://bucket/put/1' }
        ])
      }
      return { body: { id: 'note-1', chunks_stored: 2 } }
    }) as never)

    const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200 })
    vi.stubGlobal('fetch', fetchMock)

    const file = noteFile()
    await uploadChunk(file, new Uint8Array([1, 2, 3]), 0, 0, new Api())
    // Only one manifest for the file, however many chunks ask for one.
    expect(make.mock.calls.filter(([, path]) => String(path).endsWith('/upload-urls'))).toHaveLength(
      1
    )
    expect(make.mock.calls.some(([, path]) => String(path).endsWith('/finalize'))).toBe(false)

    await uploadChunk(file, new Uint8Array([4, 5, 6]), 1, 0, new Api())

    expect(fetchMock.mock.calls.map(([url]) => url)).toEqual([
      'https://bucket/put/0',
      'https://bucket/put/1'
    ])
    for (const [, init] of fetchMock.mock.calls) {
      expect(init.method).toBe('PUT')
      expect(init.credentials).toBe('omit')
    }
    expect(make.mock.calls.some(([, path]) => String(path).endsWith('/finalize'))).toBe(true)
  })

  it('relays when the server will not sign the URLs', async () => {
    setDirectTransfer(true)
    const make = vi.spyOn(Api.prototype, 'make').mockImplementation((async (
      _method: string,
      path: string
    ) => {
      if (path.endsWith('/upload-urls')) throw new Error('direct_transfer_unavailable')
      return { body: { id: 'note-1', chunks_stored: 1 } }
    }) as never)
    const fetchMock = vi.fn()
    vi.stubGlobal('fetch', fetchMock)

    await uploadChunk(noteFile(), new Uint8Array([1, 2, 3]), 0, 0, new Api())

    expect(fetchMock).not.toHaveBeenCalled()
    expect(make.mock.calls.some(([, path]) => String(path) === '/api/storage/note-1')).toBe(true)
  })

  // The bug this exists for: a note save declared the file's `size`, which is
  // the *active* version's, so every URL was signed for the previous content's
  // length and the bucket answered 403 — which reaches the page as an opaque
  // CORS failure, because an error response carries no CORS headers.
  it('declares the pending edit\'s size and chunk count, not the active version\'s', async () => {
    setDirectTransfer(true)
    const make = vi.spyOn(Api.prototype, 'make').mockImplementation((async (
      _method: string,
      path: string
    ) => {
      if (path.endsWith('/upload-urls')) {
        return manifest([
          { chunk: 0, url: 'https://bucket/put/0' },
          { chunk: 1, url: 'https://bucket/put/1' }
        ])
      }
      return { body: { id: 'note-1', chunks_stored: 2 } }
    }) as never)
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, status: 200 }))

    const midEdit = {
      ...noteFile(),
      // What the reader still sees.
      size: 11,
      chunks: 1,
      // What this save is writing.
      pending_size: 4 * 1024 * 1024 + 20,
      pending_chunks: 2
    } as unknown as UploadAppFile

    await uploadChunk(midEdit, new Uint8Array([1]), 0, 0, new Api())

    const [, , , body] = make.mock.calls.find(([, path]) =>
      String(path).endsWith('/upload-urls')
    ) as unknown[]
    expect(body).toEqual({
      chunks: [
        { chunk: 0, size: 4 * 1024 * 1024 + 16 },
        { chunk: 1, size: 20 + 16 }
      ]
    })

    // One chunk of two: nothing to commit yet.
    expect(make.mock.calls.some(([, path]) => String(path).endsWith('/finalize'))).toBe(false)
    await uploadChunk(midEdit, new Uint8Array([2]), 1, 0, new Api())
    expect(make.mock.calls.some(([, path]) => String(path).endsWith('/finalize'))).toBe(true)
  })

  // A note saved twice writes a new version, and the second save's URLs are
  // signed for it. Reusing the first save's manifest would put the new bytes
  // in the old version's slot.
  it('asks for a fresh manifest for the next upload of the same file', async () => {
    setDirectTransfer(true)
    const make = vi.spyOn(Api.prototype, 'make').mockImplementation((async (
      _method: string,
      path: string
    ) => {
      if (path.endsWith('/upload-urls')) {
        return manifest([{ chunk: 0, url: 'https://bucket/put/0' }])
      }
      return { body: { id: 'note-1', chunks_stored: 1 } }
    }) as never)
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true, status: 200 }))

    const file = { ...noteFile(), chunks: 1 } as UploadAppFile
    await uploadChunk(file, new Uint8Array([1]), 0, 0, new Api())
    await uploadChunk(file, new Uint8Array([2]), 0, 0, new Api())

    expect(make.mock.calls.filter(([, path]) => String(path).endsWith('/upload-urls'))).toHaveLength(
      2
    )
  })
})
