import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

import Api from '../services/api'
import { capabilitiesStore } from '../services/shares/capabilities'
import { fileChunkUrls, clearChunkUrlCache } from '../services/storage/download/direct'

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
})
