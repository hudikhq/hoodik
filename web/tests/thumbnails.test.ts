import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { flushPromises, mount } from '@vue/test-utils'

import * as storageMeta from '../services/storage/meta'
import * as linksMeta from '../services/links/meta'
import { store as storageStore } from '../services/storage'
import { store as linksStore } from '../services/links'
import * as cryptfns from '../services/cryptfns'
import FileThumbnail from '../src/components/files/FileThumbnail.vue'
import type { AppFile, AppLink } from '../types'

const CIPHER = 'aegis128l'
const THUMBNAIL = 'data:image/png;base64,thumb'

function baseFile(overrides: Partial<AppFile> = {}): AppFile {
  return {
    id: '11111111-1111-1111-1111-111111111111',
    user_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    is_owner: true,
    name: 'photo.jpg',
    name_hash: '',
    mime: 'image/jpeg',
    chunks: 1,
    file_id: null,
    file_modified_at: 0,
    created_at: 0,
    is_new: false,
    editable: false,
    active_version: 1,
    encrypted_key: '',
    encrypted_name: '',
    cipher: CIPHER,
    ...overrides
  } as unknown as AppFile
}

beforeEach(() => {
  setActivePinia(createPinia())
  localStorage.clear()
})

afterEach(() => {
  vi.restoreAllMocks()
})

describe('storage store: loadThumbnail', () => {
  it('fetches, decrypts and caches the thumbnail onto the store row', async () => {
    const key = await cryptfns.cipher.generateKey(CIPHER)
    const encrypted = await cryptfns.cipher.encryptString(CIPHER, THUMBNAIL, key)
    const fetcher = vi.spyOn(storageMeta, 'thumbnail').mockResolvedValue(encrypted)

    const Storage = storageStore()
    const file = baseFile({ key, has_thumbnail: true })
    Storage.addItem(file)

    const thumbnail = await Storage.loadThumbnail(file)

    expect(thumbnail).toBe(THUMBNAIL)
    expect(fetcher).toHaveBeenCalledWith(file.id)
    expect(Storage.getItem(file.id)?.thumbnail).toBe(THUMBNAIL)

    // A second call resolves from the store row without another request.
    await Storage.loadThumbnail(file)
    expect(fetcher).toHaveBeenCalledTimes(1)
  })

  it('collapses concurrent fetches for the same file into one request', async () => {
    const key = await cryptfns.cipher.generateKey(CIPHER)
    const encrypted = await cryptfns.cipher.encryptString(CIPHER, THUMBNAIL, key)
    const fetcher = vi.spyOn(storageMeta, 'thumbnail').mockResolvedValue(encrypted)

    const Storage = storageStore()
    const file = baseFile({ key, has_thumbnail: true })

    const [first, second] = await Promise.all([
      Storage.loadThumbnail(file),
      Storage.loadThumbnail(file)
    ])

    expect(first).toBe(THUMBNAIL)
    expect(second).toBe(THUMBNAIL)
    expect(fetcher).toHaveBeenCalledTimes(1)
  })

  it('never fetches for files without a thumbnail or without a key', async () => {
    const fetcher = vi.spyOn(storageMeta, 'thumbnail')

    const Storage = storageStore()
    const key = await cryptfns.cipher.generateKey(CIPHER)

    expect(await Storage.loadThumbnail(baseFile({ key }))).toBeUndefined()
    expect(await Storage.loadThumbnail(baseFile({ has_thumbnail: true }))).toBeUndefined()
    expect(fetcher).not.toHaveBeenCalled()
  })

  it('serves the encrypted blob from localStorage across sessions', async () => {
    const key = await cryptfns.cipher.generateKey(CIPHER)
    const encrypted = await cryptfns.cipher.encryptString(CIPHER, THUMBNAIL, key)
    const fetcher = vi.spyOn(storageMeta, 'thumbnail').mockResolvedValue(encrypted)

    const file = baseFile({ key, has_thumbnail: true })
    await storageStore().loadThumbnail(file)
    expect(fetcher).toHaveBeenCalledTimes(1)

    // A fresh store (new session) decrypts from the cached ciphertext
    // without touching the network.
    setActivePinia(createPinia())
    expect(await storageStore().loadThumbnail(file)).toBe(THUMBNAIL)
    expect(fetcher).toHaveBeenCalledTimes(1)
  })

  it('refetches when the file version changed', async () => {
    const key = await cryptfns.cipher.generateKey(CIPHER)
    const encrypted = await cryptfns.cipher.encryptString(CIPHER, THUMBNAIL, key)
    const fetcher = vi.spyOn(storageMeta, 'thumbnail').mockResolvedValue(encrypted)

    await storageStore().loadThumbnail(baseFile({ key, has_thumbnail: true }))

    // An edit bumped the version — the stale cache entry must not serve.
    setActivePinia(createPinia())
    await storageStore().loadThumbnail(baseFile({ key, has_thumbnail: true, active_version: 2 }))
    expect(fetcher).toHaveBeenCalledTimes(2)
  })
})

describe('links store: loadThumbnail', () => {
  it('fetches the link metadata and decrypts with the link key', async () => {
    const linkKey = await cryptfns.aes.generateKey()
    const encrypted = await cryptfns.cipher.encryptString('ascon128a', THUMBNAIL, linkKey)
    const fetcher = vi
      .spyOn(linksMeta, 'encryptedMetadata')
      .mockResolvedValue({ encrypted_thumbnail: encrypted } as never)

    const Links = linksStore()
    const link = {
      id: '22222222-2222-2222-2222-222222222222',
      has_thumbnail: true,
      link_key: linkKey
    } as unknown as AppLink

    expect(await Links.loadThumbnail(link)).toBe(THUMBNAIL)
    expect(fetcher).toHaveBeenCalledWith(link.id)
  })

  it('never fetches when the listing advertises no thumbnail', async () => {
    const fetcher = vi.spyOn(linksMeta, 'encryptedMetadata')

    const Links = linksStore()
    const link = {
      id: '22222222-2222-2222-2222-222222222222',
      link_key: await cryptfns.aes.generateKey()
    } as unknown as AppLink

    expect(await Links.loadThumbnail(link)).toBeUndefined()
    expect(fetcher).not.toHaveBeenCalled()
  })
})

describe('FileThumbnail component', () => {
  it('shows a placeholder while fetching, then swaps in the image', async () => {
    const key = await cryptfns.cipher.generateKey(CIPHER)
    const encrypted = await cryptfns.cipher.encryptString(CIPHER, THUMBNAIL, key)
    vi.spyOn(storageMeta, 'thumbnail').mockResolvedValue(encrypted)

    const wrapper = mount(FileThumbnail, {
      props: { file: baseFile({ key, has_thumbnail: true }) }
    })

    expect(wrapper.find('[name="thumbnail-placeholder"]').exists()).toBe(true)
    expect(wrapper.find('img[name="thumbnail"]').exists()).toBe(false)

    await flushPromises()

    const img = wrapper.find('img[name="thumbnail"]')
    expect(img.exists()).toBe(true)
    expect(img.attributes('src')).toBe(THUMBNAIL)
    expect(wrapper.find('[name="thumbnail-placeholder"]').exists()).toBe(false)
  })

  it('renders an already-decrypted thumbnail without any fetch', async () => {
    const fetcher = vi.spyOn(storageMeta, 'thumbnail')

    const wrapper = mount(FileThumbnail, {
      props: { file: baseFile({ thumbnail: THUMBNAIL, has_thumbnail: true }) }
    })
    await flushPromises()

    expect(wrapper.find('img[name="thumbnail"]').attributes('src')).toBe(THUMBNAIL)
    expect(fetcher).not.toHaveBeenCalled()
  })

  it('renders the fallback slot for files without a thumbnail', async () => {
    const wrapper = mount(FileThumbnail, {
      props: { file: baseFile() },
      slots: { default: '<span data-testid="fallback-icon" />' }
    })
    await flushPromises()

    expect(wrapper.find('[data-testid="fallback-icon"]').exists()).toBe(true)
    expect(wrapper.find('img[name="thumbnail"]').exists()).toBe(false)
  })

  it('keeps the fallback when the fetch fails', async () => {
    vi.spyOn(storageMeta, 'thumbnail').mockRejectedValue(new Error('offline'))

    const key = await cryptfns.cipher.generateKey(CIPHER)
    const wrapper = mount(FileThumbnail, {
      props: { file: baseFile({ key, has_thumbnail: true }) },
      slots: { default: '<span data-testid="fallback-icon" />' }
    })
    await flushPromises()

    expect(wrapper.find('img[name="thumbnail"]').exists()).toBe(false)
    expect(wrapper.find('[data-testid="fallback-icon"]').exists()).toBe(true)
  })
})
