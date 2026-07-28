import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import * as storageMeta from '../services/storage/meta'
import { store as storageStore } from '../services/storage'
import type { KeyPair } from '../services/cryptfns/rsa'

const KEYPAIR = {} as KeyPair

beforeEach(() => {
  setActivePinia(createPinia())
  vi.restoreAllMocks()
})

describe('storage store — listing failure', () => {
  it('exposes the message so the browser can render it', async () => {
    vi.spyOn(storageMeta, 'find').mockRejectedValue(new Error('network is down'))

    const store = storageStore()
    await store.find(KEYPAIR, undefined)

    expect(store.error).toContain('network is down')
  })

  it('clears the message on the next successful listing', async () => {
    const find = vi.spyOn(storageMeta, 'find').mockRejectedValue(new Error('network is down'))

    const store = storageStore()
    await store.find(KEYPAIR, undefined)
    expect(store.error).not.toBeNull()

    find.mockResolvedValue({ children: [], parents: [] })
    await store.find(KEYPAIR, undefined)

    expect(store.error).toBeNull()
  })
})
