import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'

import AsideFileTree from '../src/components/ui/AsideFileTree.vue'
import * as meta from '../services/storage/meta'
import * as sharesApi from '../services/shares/api'
import { store as filesStore } from '../services/storage'

import type { AppFile, IncomingSharePage, KeyPair } from '../types'

const EMPTY_PAGE: IncomingSharePage = { items: [], total: 0, limit: 50, offset: 0 }

const kp = {
  input: 'priv',
  publicKey: 'pub',
  fingerprint: 'fp-seed'
} as unknown as KeyPair

function rootDir(): AppFile {
  return {
    id: 'dddddddd-dddd-dddd-dddd-dddddddddddd',
    file_id: null,
    mime: 'dir',
    encrypted_name: 'cipher',
    chunks: 0
  } as unknown as AppFile
}

function setupRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/files/:file_id?', name: 'files', component: { template: '<div />' } }]
  })
}

beforeEach(() => {
  setActivePinia(createPinia())
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('AsideFileTree first load at root', () => {
  it('seeds from the main listing instead of fetching its own copy', async () => {
    const find = vi.spyOn(meta, 'find').mockResolvedValue({ children: [rootDir()], parents: [] })
    vi.spyOn(meta, 'decrypt').mockResolvedValue({ name: 'Documents' } as never)
    const shares = vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(EMPTY_PAGE)

    const router = setupRouter()
    await router.push('/files')
    await router.isReady()

    // Tree mounts before the main view's listing lands — the store race
    // this dedupe exists for.
    const wrapper = mount(AsideFileTree, {
      props: { keypair: kp },
      global: { plugins: [router] }
    })

    await filesStore().find(kp, undefined)
    await flushPromises()

    // One listing and one shares probe total — both the store's; the tree
    // fetched nothing of its own.
    expect(find).toHaveBeenCalledTimes(1)
    expect(shares).toHaveBeenCalledTimes(1)
    expect(wrapper.text()).toContain('Documents')

    wrapper.unmount()
  })
})
