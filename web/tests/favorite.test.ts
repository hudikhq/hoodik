import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

import * as storageMeta from '../services/storage/meta'
import { store as storageStore, SHARED_WITH_ME_DIR_ID } from '../services/storage'
import type { AppFile, KeyPair } from '../types'

const KEYPAIR = {} as KeyPair

function file(id: string, favorite = false): AppFile {
  return {
    id,
    name: id,
    mime: 'text/plain',
    favorite,
    is_owner: true,
    is_new: false,
    editable: false,
    active_version: 1,
    chunks: 0,
    file_modified_at: 0,
    created_at: 0,
    name_hash: '',
    encrypted_name: '',
    encrypted_key: '',
    cipher: 'aegis128l'
  } as unknown as AppFile
}

beforeEach(() => {
  setActivePinia(createPinia())
  vi.restoreAllMocks()
})

describe('storage store — favourites', () => {
  it('sets the flag on a batch through the many endpoint', async () => {
    const many = vi.spyOn(storageMeta, 'favoriteMany').mockResolvedValue(undefined)

    const store = storageStore()
    store.upsertItem(file('a'))
    store.upsertItem(file('b'))

    await store.setFavorite(KEYPAIR, [file('a'), file('b')], true)

    expect(many).toHaveBeenCalledWith({ ids: ['a', 'b'], favorite: true })
    expect(store.getItem('a')?.favorite).toBe(true)
    expect(store.getItem('b')?.favorite).toBe(true)
  })

  it('clears the flag on a batch too', async () => {
    vi.spyOn(storageMeta, 'favoriteMany').mockResolvedValue(undefined)

    const store = storageStore()
    store.upsertItem(file('a', true))
    store.upsertItem(file('b', true))

    await store.setFavorite(KEYPAIR, [file('a', true), file('b', true)], false)

    expect(store.getItem('a')?.favorite).toBe(false)
    expect(store.getItem('b')?.favorite).toBe(false)
  })

  it('uses the single endpoint for one file and keeps the store row in sync', async () => {
    const single = vi.spyOn(storageMeta, 'setFavorite').mockResolvedValue(file('a', true))
    const kp = { input: 'private-key' } as unknown as KeyPair

    const store = storageStore()
    store.upsertItem({ ...file('a'), key: new Uint8Array([1]) })

    await store.setFavorite(kp, [file('a')], true)

    expect(single).toHaveBeenCalledWith(kp, 'a', true)
    expect(store.getItem('a')?.favorite).toBe(true)
  })

  it('skips the synthetic "Shared with me" root', async () => {
    const many = vi.spyOn(storageMeta, 'favoriteMany').mockResolvedValue(undefined)

    const store = storageStore()
    const real = file('a')

    await store.setFavorite(KEYPAIR, [file(SHARED_WITH_ME_DIR_ID), real], true)

    expect(many).toHaveBeenCalledWith({ ids: ['a'], favorite: true })
  })

  it('does nothing when the selection is only the synthetic root', async () => {
    const many = vi.spyOn(storageMeta, 'favoriteMany').mockResolvedValue(undefined)

    const store = storageStore()
    await store.setFavorite(KEYPAIR, [file(SHARED_WITH_ME_DIR_ID)], true)

    expect(many).not.toHaveBeenCalled()
  })

  it('pins favourites above the rest, keeping the chosen sort within each group', () => {
    const store = storageStore()
    const row = (id: string, mime: string, favorite = false) =>
      ({ ...file(id, favorite), mime, file_id: null } as AppFile)

    store.upsertItem(row('a', 'dir'))
    store.upsertItem(row('b', 'dir', true))
    store.upsertItem(row('c', 'text/plain', true))
    store.upsertItem(row('d', 'text/plain'))
    store.upsertItem(row('e', 'text/plain', true))
    store.setSort('root', 'name', 'desc')

    expect(store.items.map((f) => f.id)).toEqual(['b', 'e', 'c', 'a', 'd'])
  })
})
