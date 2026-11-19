import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'

// The link-view chain pulls in services/preview/link which calls the
// Links pinia store at module load. Activate pinia before the dynamic
// import so the side effect lands on a real store.
setActivePinia(createPinia())

import { ErrorResponse } from '../services/api'
import type { LinksStore } from '../types'

let LinkViewInner: typeof import('../src/views/links/link-view/LinkViewInner.vue').default

beforeAll(async () => {
  LinkViewInner = (await import('../src/views/links/link-view/LinkViewInner.vue')).default
})

function fakeError(status: number, message = 'gone'): ErrorResponse<unknown> {
  return Object.assign(new Error(message), {
    kind: 'ErrorResponse',
    status,
    description: message,
    body: { message },
    request: { method: 'get', url: '/api/links/x/metadata' },
    headers: new Headers(),
    rawBody: undefined,
    validation: null
  }) as unknown as ErrorResponse<unknown>
}

function fakeLinksStore(getImpl: () => Promise<never>): LinksStore {
  return {
    get: vi.fn(getImpl)
  } as unknown as LinksStore
}

const router = createRouter({
  history: createMemoryHistory(),
  routes: [{ path: '/:catchAll(.*)', component: { template: '<div />' } }]
})

async function mountInner(linkKeyHex: string | undefined, store: LinksStore) {
  // LinkViewInner's `await load()` makes it an async setup component;
  // testing it directly needs a Suspense parent so the resolved tree
  // renders before assertions.
  const Wrapper = {
    components: { LinkViewInner },
    props: ['linkKeyHex'],
    setup(props: { linkKeyHex?: string }) {
      return { props, store }
    },
    template: `
      <Suspense>
        <LinkViewInner :Links="store" id="link-id" :linkKeyHex="props.linkKeyHex" />
      </Suspense>
    `
  }
  const wrapper = mount(Wrapper, {
    global: { plugins: [router] },
    props: { linkKeyHex }
  })
  await flushPromises()
  return wrapper
}

beforeEach(() => {
  setActivePinia(createPinia())
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('LinkViewInner — revoked / unavailable link handling', () => {
  it('renders_unavailable_panel_when_metadata_endpoint_404s', async () => {
    const store = fakeLinksStore(() => Promise.reject(fakeError(404, 'link gone')))
    const wrapper = await mountInner('deadbeef', store)
    await flushPromises()
    expect(wrapper.find('[data-testid="link-unavailable"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Link expired or removed')
  })

  it('renders_unlock_form_when_no_link_key_is_present_in_url', async () => {
    // No key in fragment = the user typed the URL without the #linkKey
    // suffix. The unlock form prompts them for it; we never call the
    // backend on this branch, so no error pathway runs.
    const store = fakeLinksStore(() => Promise.reject(fakeError(404)))
    const wrapper = await mountInner(undefined, store)
    await flushPromises()
    expect(wrapper.find('[data-testid="link-unavailable"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('Unlock The Link')
  })

  it('keeps_unlock_form_when_load_fails_with_non_404', async () => {
    // A 400 / 500 or a decrypt failure leaves the unlock form on screen
    // with the error inline — the user can retype the key without
    // navigating away.
    const store = fakeLinksStore(() => Promise.reject(fakeError(400, 'bad key')))
    const wrapper = await mountInner('badkey', store)
    await flushPromises()
    expect(wrapper.find('[data-testid="link-unavailable"]').exists()).toBe(false)
    expect(wrapper.text()).toContain('Unlock The Link')
  })
})

describe('LinkViewInner — download feedback', () => {
  const resolvedLink = {
    id: 'link-id',
    file_id: 'file-id',
    name: 'big-archive.zip',
    file_size: 1000,
    file_mime: 'application/zip',
    link_key_hex: 'deadbeef',
    expires_at: undefined
  }

  function deferred() {
    let resolve!: () => void
    let reject!: (e: unknown) => void
    const promise = new Promise<void>((res, rej) => {
      resolve = res
      reject = rej
    })
    return { promise, resolve, reject }
  }

  function downloadStore(pending: Promise<void>) {
    let onBytes: ((bytes: number) => void) | undefined
    const store = {
      get: vi.fn(() => Promise.resolve(resolvedLink)),
      formDownload: vi.fn((_id: string, _key: string, cb?: (bytes: number) => void) => {
        onBytes = cb
        return pending
      })
    } as unknown as LinksStore
    return { store, report: (bytes: number) => onBytes?.(bytes) }
  }

  it('shows_waiting_overlay_with_progress_until_the_save_starts', async () => {
    const pending = deferred()
    const { store, report } = downloadStore(pending.promise)
    const wrapper = await mountInner('deadbeef', store)

    expect(wrapper.find('[data-testid="link-download-progress"]').exists()).toBe(false)

    await wrapper.find('[name="preview-download"]').trigger('click')
    await flushPromises()

    const overlay = wrapper.find('[data-testid="link-download-progress"]')
    expect(overlay.exists()).toBe(true)
    expect(overlay.text()).toContain('Stand by, download will start soon')

    report(500)
    await flushPromises()
    expect(wrapper.find('[data-testid="link-download-progress"]').text()).toContain('50 %')

    pending.resolve()
    await flushPromises()
    expect(wrapper.find('[data-testid="link-download-progress"]').exists()).toBe(false)
  })

  it('center_button_triggers_the_same_download_and_repeat_clicks_are_ignored', async () => {
    const pending = deferred()
    const { store } = downloadStore(pending.promise)
    const wrapper = await mountInner('deadbeef', store)

    const center = wrapper.find('[name="preview-download-center"]')
    expect(center.exists()).toBe(true)

    await center.trigger('click')
    await wrapper.find('[name="preview-download"]').trigger('click')
    await flushPromises()

    expect(store.formDownload).toHaveBeenCalledTimes(1)
    pending.resolve()
    await flushPromises()
  })

  it('shows_an_error_when_the_download_fails', async () => {
    const pending = deferred()
    const { store } = downloadStore(pending.promise)
    const wrapper = await mountInner('deadbeef', store)

    await wrapper.find('[name="preview-download"]').trigger('click')
    pending.reject(new Error('network down'))
    await flushPromises()

    expect(wrapper.find('[data-testid="link-download-progress"]').exists()).toBe(false)
    expect(wrapper.find('[role="alert"]').text()).toContain('network down')
  })
})
