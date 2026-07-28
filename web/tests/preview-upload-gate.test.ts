import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'
import { createMemoryHistory, createRouter } from 'vue-router'

import AsideFileTree from '../src/components/ui/AsideFileTree.vue'
import NotesLayout from '../src/views/notes/NotesLayout.vue'
import NotesEditor from '../src/views/notes/NotesEditor.vue'
import NotesLanding from '../src/views/notes/NotesLanding.vue'
import SearchModalResult from '../src/components/files/search/SearchModalResult.vue'
import { FilePreview } from '../services/preview/file'
import { LinkPreview } from '../services/preview/link'
import * as storage from '../services/storage'
import * as meta from '../services/storage/meta'
import * as sharesApi from '../services/shares/api'

import type { AppFile, AppLink, FilesStore, IncomingSharePage, KeyPair } from '../types'

const EMPTY_PAGE: IncomingSharePage = { items: [], total: 0, limit: 50, offset: 0 }

const kp = { input: 'priv', publicKey: 'pub' } as unknown as KeyPair

function makeFile(partial: Partial<AppFile>): AppFile {
  return {
    id: 'f1',
    user_id: 'u1',
    is_owner: true,
    name_hash: 'hash',
    name: 'file.png',
    mime: 'image/png',
    size: 100,
    chunks: 1,
    encrypted_key: '',
    encrypted_name: 'cipher',
    cipher: 'aegis128l',
    editable: false,
    active_version: 1,
    file_id: null,
    file_modified_at: 0,
    created_at: 0,
    finished_upload_at: 1_700_000_000,
    is_new: false,
    ...partial
  } as AppFile
}

function setupRouter() {
  return createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/files/:file_id?', name: 'files', component: { template: '<div />' } },
      { path: '/files/preview/:id', name: 'file-preview', component: { template: '<div />' } },
      { path: '/notes/:id?', name: 'notes', component: { template: '<div />' } }
    ]
  })
}

beforeEach(() => {
  setActivePinia(createPinia())
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('FilePreview upload gate', () => {
  it('UNIT: a preview built from an unfinished row is not previewable', () => {
    const preview = new FilePreview(makeFile({ finished_upload_at: undefined }), kp)

    expect(preview.is()).toBe(false)
    expect(preview.previewType()).toBeNull()
  })

  it('UNIT: a preview built from a finished row keeps its type', () => {
    const preview = new FilePreview(makeFile({}), kp)

    expect(preview.is()).toBe(true)
    expect(preview.previewType()).toBe('image')
  })

  it('UNIT: link previews are unaffected — a link target is always complete', () => {
    const link = new LinkPreview({
      id: 'l1',
      file_id: 'f1',
      name: 'doc.pdf',
      file_mime: 'application/pdf',
      file_size: 100
    } as AppLink)

    expect(link.is()).toBe(true)
    expect(link.previewType()).toBe('pdf')
  })
})

describe('AsideFileTree file click', () => {
  async function mountTree(file: AppFile, fingerprint: string) {
    vi.spyOn(meta, 'find').mockResolvedValue({ children: [file], parents: [] } as unknown as Awaited<
      ReturnType<typeof meta.find>
    >)
    vi.spyOn(meta, 'decrypt').mockResolvedValue({ name: file.name } as never)
    vi.spyOn(sharesApi, 'getSharesMine').mockResolvedValue(EMPTY_PAGE)

    // Left off the `files` route on purpose: mounting there makes the tree
    // wait for the main view's root listing, which never arrives here.
    const router = setupRouter()
    const push = vi.spyOn(router, 'push')
    const wrapper = mount(AsideFileTree, {
      props: { keypair: { ...kp, fingerprint } as unknown as KeyPair },
      global: { plugins: [router] }
    })
    await flushPromises()

    return { wrapper, push }
  }

  it('UNIT: a still-uploading file opens its folder instead of the preview', async () => {
    const file = makeFile({
      id: 'uploading',
      name: 'half.png',
      file_id: 'parent-dir',
      finished_upload_at: undefined
    })
    const { wrapper, push } = await mountTree(file, 'fp-uploading')

    await wrapper.get('li[title="half.png"]').trigger('click')

    expect(push).toHaveBeenCalledWith({ name: 'files', params: { file_id: 'parent-dir' } })
  })

  it('UNIT: a finished markdown file still opens the editor', async () => {
    const file = makeFile({ id: 'note', name: 'note.md', mime: 'text/markdown' })
    const { wrapper, push } = await mountTree(file, 'fp-finished-md')

    await wrapper.get('li[title="note.md"]').trigger('click')

    expect(push).toHaveBeenCalledWith({ name: 'notes', params: { id: 'note' } })
  })
})

describe('NotesLayout deep link', () => {
  async function mountLayout(file: AppFile) {
    vi.spyOn(meta, 'find').mockResolvedValue({ children: [], parents: [] } as unknown as Awaited<
      ReturnType<typeof meta.find>
    >)

    const Storage = { metadata: vi.fn().mockResolvedValue(file) } as unknown as FilesStore
    const router = setupRouter()
    await router.push({ name: 'notes', params: { id: file.id } })
    await router.isReady()

    const wrapper = mount(NotesLayout, {
      props: { Storage, keypair: kp, loading: false },
      global: { plugins: [router] }
    })
    await flushPromises()

    return wrapper
  }

  it('UNIT: an unfinished note refuses to mount the editor', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => undefined)

    const wrapper = await mountLayout(
      makeFile({ id: 'note', name: 'note.md', mime: 'text/markdown', finished_upload_at: undefined })
    )

    expect(wrapper.findComponent(NotesEditor).exists()).toBe(false)
  })

  it('UNIT: a finished note mounts the editor', async () => {
    const wrapper = await mountLayout(makeFile({ id: 'note', name: 'note.md', mime: 'text/markdown' }))

    expect(wrapper.findComponent(NotesEditor).exists()).toBe(true)
  })
})

describe('SearchModalResult link target', () => {
  async function mountResult(file: AppFile) {
    const router = setupRouter()
    await router.push({ name: 'files' })
    await router.isReady()

    return mount(SearchModalResult, { props: { file }, global: { plugins: [router] } })
  }

  it('UNIT: a still-uploading hit links to its folder instead of the preview', async () => {
    const wrapper = await mountResult(
      makeFile({ id: 'uploading', file_id: 'parent-dir', finished_upload_at: undefined })
    )

    expect(wrapper.get('a').attributes('href')).toBe('/files/parent-dir')
  })

  it('UNIT: a still-uploading markdown hit skips the editor too', async () => {
    const wrapper = await mountResult(
      makeFile({
        id: 'uploading-md',
        name: 'half.md',
        mime: 'text/markdown',
        file_id: 'parent-dir',
        finished_upload_at: undefined
      })
    )

    expect(wrapper.get('a').attributes('href')).toBe('/files/parent-dir')
  })

  it('UNIT: a still-uploading hit at the root links to the root listing', async () => {
    const wrapper = await mountResult(makeFile({ id: 'uploading', finished_upload_at: undefined }))

    expect(wrapper.get('a').attributes('href')).toBe('/files')
  })

  it('UNIT: a finished hit still links to the preview', async () => {
    const wrapper = await mountResult(makeFile({ id: 'done' }))

    expect(wrapper.get('a').attributes('href')).toBe('/files/preview/done')
  })
})

describe('NotesLanding listing', () => {
  async function mountLanding(files: AppFile[]) {
    vi.spyOn(meta, 'find').mockResolvedValue({ children: files, parents: [] } as unknown as Awaited<
      ReturnType<typeof meta.find>
    >)
    vi.spyOn(meta, 'decrypt').mockImplementation(
      async (item) => ({ name: (item as AppFile).name }) as never
    )

    const router = setupRouter()
    const wrapper = mount(NotesLanding, {
      props: { keypair: kp },
      global: { plugins: [router] }
    })
    await flushPromises()

    return wrapper
  }

  function titles(wrapper: ReturnType<typeof mount>): string[] {
    // Only the note rows carry a title; the create-note modal renders list
    // items of its own that would otherwise land in this projection.
    return wrapper.findAll('li[title]').map((li) => li.attributes('title') as string)
  }

  it('UNIT: an unfinished note is left out of the recent list', async () => {
    const wrapper = await mountLanding([
      makeFile({ id: 'done', name: 'done.md', mime: 'text/markdown' }),
      makeFile({
        id: 'half',
        name: 'half.md',
        mime: 'text/markdown',
        finished_upload_at: undefined
      })
    ])

    expect(titles(wrapper)).toEqual(['done.md'])
  })

  it('UNIT: an unfinished note is left out of the search results', async () => {
    const wrapper = await mountLanding([])
    const searchSpy = vi.spyOn(storage, 'search').mockResolvedValue([
      makeFile({ id: 'done', name: 'found.md', mime: 'text/markdown' }),
      makeFile({
        id: 'half',
        name: 'half.md',
        mime: 'text/markdown',
        finished_upload_at: undefined
      })
    ])

    await wrapper.get('input[type="text"]').setValue('md')
    // The query watcher debounces by 300ms before it hits the search endpoint.
    await new Promise((resolve) => setTimeout(resolve, 350))
    await flushPromises()

    expect(searchSpy).toHaveBeenCalled()
    expect(titles(wrapper)).toEqual(['found.md'])
  })
})
