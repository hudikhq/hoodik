import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

import Api from '../services/api'
import { capabilitiesStore } from '../services/shares/capabilities'
import {
  fileChunkUrls,
  versionChunkUrls,
  clearChunkUrlCache,
  evictChunkUrls
} from '../services/storage/download/direct'

function manifest(urls: { chunk: number; url: string }[], expiresInSeconds = 3600) {
  return {
    body: {
      urls,
      expires_at: Math.floor(Date.now() / 1000) + expiresInSeconds
    }
  }
}

/**
 * Put the capability store into a known state without touching the network.
 */
function setDirectTransfer(enabled: boolean) {
  const caps = capabilitiesStore()
  caps.caps = {
    sharing: { enabled: false, roles: [] },
    editable_folders: false,
    share_groups: false,
    audit_log: false,
    fork: false,
    direct_transfer: enabled
  }
}

describe('direct chunk-url manifests', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    clearChunkUrlCache()
    vi.restoreAllMocks()
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('asks for nothing when the server does not advertise direct transfer', async () => {
    setDirectTransfer(false)
    const get = vi.spyOn(Api, 'get')

    expect(await fileChunkUrls('file-1')).toBeUndefined()
    expect(get).not.toHaveBeenCalled()
  })

  it('orders URLs by chunk index rather than by response order', async () => {
    setDirectTransfer(true)
    vi.spyOn(Api, 'get').mockResolvedValue(
      manifest([
        { chunk: 2, url: 'https://bucket/2' },
        { chunk: 0, url: 'https://bucket/0' },
        { chunk: 1, url: 'https://bucket/1' }
      ]) as never
    )

    expect(await fileChunkUrls('file-ordered')).toEqual([
      'https://bucket/0',
      'https://bucket/1',
      'https://bucket/2'
    ])
  })

  // A progressive consumer builds one downloader per chunk, so without the
  // cache a video would refetch the whole manifest for every chunk it plays.
  it('fetches once and serves later calls from cache', async () => {
    setDirectTransfer(true)
    const get = vi
      .spyOn(Api, 'get')
      .mockResolvedValue(manifest([{ chunk: 0, url: 'https://bucket/0' }]) as never)

    await fileChunkUrls('file-cached')
    await fileChunkUrls('file-cached')
    await fileChunkUrls('file-cached')

    expect(get).toHaveBeenCalledTimes(1)
  })

  // Signed URLs that expire mid-transfer are worse than no URLs at all, so a
  // manifest inside the safety margin is dropped and re-fetched.
  it('refetches a manifest that is about to expire', async () => {
    setDirectTransfer(true)
    const get = vi
      .spyOn(Api, 'get')
      .mockResolvedValue(manifest([{ chunk: 0, url: 'https://bucket/0' }], 5) as never)

    await fileChunkUrls('file-stale')
    await fileChunkUrls('file-stale')

    expect(get).toHaveBeenCalledTimes(2)
  })

  it('falls back to the server path when the manifest call fails', async () => {
    setDirectTransfer(true)
    vi.spyOn(Api, 'get').mockRejectedValue(new Error('boom'))

    expect(await fileChunkUrls('file-broken')).toBeUndefined()
  })

  it('treats an empty manifest as no manifest', async () => {
    setDirectTransfer(true)
    vi.spyOn(Api, 'get').mockResolvedValue(manifest([]) as never)

    expect(await fileChunkUrls('file-empty')).toBeUndefined()
  })

  // A manifest describes the version that was active when it was signed, and
  // the URLs stay valid for days. Held across a content replace, the next
  // download serves the old version's chunks — or a mix of two, which fails to
  // decrypt rather than merely being stale.
  it('forgets a file manifest when its content changes', async () => {
    setDirectTransfer(true)
    const get = vi
      .spyOn(Api, 'get')
      .mockResolvedValue(manifest([{ chunk: 0, url: 'https://bucket/v1-0' }]) as never)

    expect(await fileChunkUrls('file-9')).toEqual(['https://bucket/v1-0'])
    expect(get).toHaveBeenCalledTimes(1)

    // Cached: no second request.
    await fileChunkUrls('file-9')
    expect(get).toHaveBeenCalledTimes(1)

    evictChunkUrls('file-9')

    get.mockResolvedValue(manifest([{ chunk: 0, url: 'https://bucket/v2-0' }]) as never)
    expect(await fileChunkUrls('file-9')).toEqual(['https://bucket/v2-0'])
    expect(get).toHaveBeenCalledTimes(2)
  })

  it('forgets that file\'s historical versions too, and nothing else', async () => {
    setDirectTransfer(true)
    const get = vi
      .spyOn(Api, 'get')
      .mockResolvedValue(manifest([{ chunk: 0, url: 'https://bucket/a' }]) as never)

    await versionChunkUrls('file-9', 3)
    await fileChunkUrls('other-file')
    expect(get).toHaveBeenCalledTimes(2)

    evictChunkUrls('file-9')

    // The other file keeps its manifest; only file-9's went.
    await fileChunkUrls('other-file')
    expect(get).toHaveBeenCalledTimes(2)

    await versionChunkUrls('file-9', 3)
    expect(get).toHaveBeenCalledTimes(3)
  })
})
