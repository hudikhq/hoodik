import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'
import { createRouter, createMemoryHistory } from 'vue-router'

import AsideFileTree from '../../src/components/ui/AsideFileTree.vue'
import * as meta from '../../services/storage/meta'
import * as sharesApi from '../../services/shares/api'
import { SHARED_WITH_ME_DIR_ID } from '../../services/storage'

import type { IncomingSharePage, KeyPair } from '../../types'

// The AsideFileTree component keeps reactive state at module scope so it
// can survive sidebar collapse + remount. Each test feeds a distinct
// fingerprint so the watcher trips a fresh `loadRoot` instead of
// short-circuiting against the previous run's snapshot.
function keypair(fingerprint: string): KeyPair {
  return {
    input: 'priv',
    publicKey: 'pub',
    fingerprint
  } as unknown as KeyPair
}

function setupRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/files/:file_id?', name: 'files', component: { template: '<div />' } }]
  })
}

function makeIncomingPage(): IncomingSharePage {
  return {
    items: [
      {
        file_id: '11111111-1111-1111-1111-111111111111',
        mime: 'text/plain',
        encrypted_name: 'cipher',
        cipher: 'aegis128l',
        editable: false,
        share_role: 'reader',
        encrypted_key: 'wrapped',
        created_at: 1_700_000_000,
        shared_at: 1_700_000_500,
        owner_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
        owner_email: 'alice@example.com',
        owner_pubkey: 'pub',
        owner_pubkey_fingerprint: 'fp',
        shared_by_user_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
        shared_by_email: 'alice@example.com'
      }
    ],
    total: 1,
    limit: 50,
    offset: 0
  }
}

const EMPTY_PAGE: IncomingSharePage = { items: [], total: 0, limit: 50, offset: 0 }

beforeEach(() => {
  setActivePinia(createPinia())
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('AsideFileTree: Shared with me sidebar entry', () => {
  it('aside_tree_renders_shared_with_me_when_incoming_share_exists', async () => {
    const kp = keypair('fp-renders')
    vi.spyOn(meta, 'find').mockResolvedValue({ children: [], parents: [] })
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())

    const router = setupRouter()
    const wrapper = mount(AsideFileTree, {
      props: { keypair: kp },
      global: { plugins: [router] }
    })
    await flushPromises()

    const entry = wrapper.find('[data-testid="aside-tree-shared-with-me"]')
    expect(entry.exists()).toBe(true)
    expect(entry.text()).toContain('Shared with me')
  })

  it('aside_tree_omits_shared_with_me_when_no_incoming_share', async () => {
    const kp = keypair('fp-omits')
    vi.spyOn(meta, 'find').mockResolvedValue({ children: [], parents: [] })
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(EMPTY_PAGE)

    const router = setupRouter()
    const wrapper = mount(AsideFileTree, {
      props: { keypair: kp },
      global: { plugins: [router] }
    })
    await flushPromises()

    expect(wrapper.find('[data-testid="aside-tree-shared-with-me"]').exists()).toBe(false)
  })

  it('aside_tree_shared_with_me_renders_expand_chevron', async () => {
    // The synthetic node renders chevron + folder icon, same shape as a
    // regular expandable folder. Clicking the row expands it into the
    // recipient's incoming-share roots.
    const kp = keypair('fp-chevron')
    vi.spyOn(meta, 'find').mockResolvedValue({ children: [], parents: [] })
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())

    const router = setupRouter()
    const wrapper = mount(AsideFileTree, {
      props: { keypair: kp },
      global: { plugins: [router] }
    })
    await flushPromises()

    const entry = wrapper.find('[data-testid="aside-tree-shared-with-me"]')
    expect(entry.exists()).toBe(true)
    expect(entry.findAll('svg').length).toBe(2)
  })

  it('aside_tree_click_on_shared_with_me_navigates_to_virtual_folder', async () => {
    const kp = keypair('fp-click')
    vi.spyOn(meta, 'find').mockResolvedValue({ children: [], parents: [] })
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())

    const router = setupRouter()
    const pushSpy = vi.spyOn(router, 'push')
    const wrapper = mount(AsideFileTree, {
      props: { keypair: kp },
      global: { plugins: [router] }
    })
    await flushPromises()

    await wrapper.get('[data-testid="aside-tree-shared-with-me"]').trigger('click')
    await flushPromises()

    expect(pushSpy).toHaveBeenCalledWith({
      name: 'files',
      params: { file_id: SHARED_WITH_ME_DIR_ID }
    })
  })

  it('aside_tree_expansion_loads_children_from_shares_api', async () => {
    // The recipient's owned-root listing comes from `meta.find` (called on
    // mount). The synthetic node's expansion has to fetch its children
    // from `/api/shares/mine` instead — otherwise the recipient's own
    // roots would surface again as the synthetic node's children, which
    // is the bug this test pins. Verify the expansion does NOT trigger
    // another `meta.find` and that the share row is rendered.
    const kp = keypair('fp-expand-incoming')
    const findSpy = vi.spyOn(meta, 'find').mockResolvedValue({
      children: [
        {
          id: 'owned-root-a',
          file_id: null,
          mime: 'dir',
          name: 'own-encrypted',
          encrypted_name: 'owned-cipher',
          encrypted_key: 'owned-wrap',
          cipher: 'aegis128l'
        }
      ],
      parents: []
    } as unknown as Awaited<ReturnType<typeof meta.find>>)
    vi.spyOn(meta, 'decrypt').mockImplementation(async (item) => {
      const wrap = (item as { encrypted_key?: string }).encrypted_key
      if (wrap === 'owned-wrap') return { name: 'own-root' }
      if (wrap === 'wrapped') return { name: 'shared-doc.txt' }
      return { name: '' }
    })
    const sharesSpy = vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())

    const router = setupRouter()
    const wrapper = mount(AsideFileTree, {
      props: { keypair: kp },
      global: { plugins: [router] }
    })
    await flushPromises()

    findSpy.mockClear()
    sharesSpy.mockClear()
    const entry = wrapper.get('[data-testid="aside-tree-shared-with-me"]')
    // Module-level treeState carries the expanded set across tests in
    // this file. If a previous test left the synthetic id flagged as
    // expanded, the first click here collapses it; the follow-up click
    // is what actually triggers the lazy load. Doing both keeps the test
    // robust against run order.
    await entry.trigger('click')
    await flushPromises()
    if (!wrapper.text().includes('shared-doc.txt')) {
      await entry.trigger('click')
      await flushPromises()
    }

    expect(findSpy).not.toHaveBeenCalled()
    expect(sharesSpy).toHaveBeenCalled()
    expect(wrapper.text()).toContain('shared-doc.txt')
  })

  it('aside_tree_expansion_with_no_incoming_renders_empty_state', async () => {
    // Mounting only renders the synthetic node when the recipient has at
    // least one incoming share, so we expose `loadSharedRoots` through the
    // expansion path with a non-empty initial fetch but an empty
    // follow-up. The component is wired to use the same `getSharesMine`
    // call for both purposes; the empty state surfaces when the second
    // call returns no rows.
    const kp = keypair('fp-empty-state')
    vi.spyOn(meta, 'find').mockResolvedValue({ children: [], parents: [] })
    const sharesSpy = vi
      .spyOn(sharesApi, 'getSharesMine')
      .mockResolvedValueOnce(makeIncomingPage())
      .mockResolvedValue(EMPTY_PAGE)

    const router = setupRouter()
    const wrapper = mount(AsideFileTree, {
      props: { keypair: kp },
      global: { plugins: [router] }
    })
    await flushPromises()

    const entry = wrapper.get('[data-testid="aside-tree-shared-with-me"]')
    await entry.trigger('click')
    await flushPromises()
    if (!wrapper.find('[data-testid="aside-tree-shared-with-me-empty"]').exists()) {
      await entry.trigger('click')
      await flushPromises()
    }

    expect(sharesSpy.mock.calls.length).toBeGreaterThanOrEqual(2)
    expect(wrapper.find('[data-testid="aside-tree-shared-with-me-empty"]').exists()).toBe(true)
  })

  it('aside_tree_shared_with_me_pins_first', async () => {
    const kp = keypair('fp-pins-first')
    vi.spyOn(meta, 'find').mockResolvedValue({
      children: [
        {
          id: 'dir-academica',
          file_id: null,
          mime: 'dir',
          name: 'academica.lux',
          encrypted_name: '',
          encrypted_key: '',
          cipher: 'aegis128l'
        }
      ],
      parents: []
    } as unknown as Awaited<ReturnType<typeof meta.find>>)
    vi.spyOn(meta, 'decrypt').mockResolvedValue({ name: 'academica.lux' })
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(makeIncomingPage())

    const router = setupRouter()
    const wrapper = mount(AsideFileTree, {
      props: { keypair: kp },
      global: { plugins: [router] }
    })
    await flushPromises()

    const rows = wrapper.findAll('li')
    expect(rows.length).toBeGreaterThan(1)
    expect(rows[0].text()).toContain('Shared with me')
  })
})
