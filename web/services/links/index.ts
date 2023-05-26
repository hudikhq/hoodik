import { defineStore } from 'pinia'
import { ref } from 'vue'
import Api from '!/api'
import * as meta from './meta'
import * as crypto from './crypto'
import type { AppLink, CreateLink, EncryptedAppLink, KeyPair, ListAppFile } from 'types'
import { utcStringFromLocal } from '..'

export { meta, crypto }

export const store = defineStore('links', () => {
  const loading = ref(false)

  const items = ref<AppLink[]>([])

  /**
   * Files selected to be deleted from various places
   */
  const forDelete = ref<AppLink[]>([])

  /**
   * Add single file to select list
   */
  function selectOne(select: boolean, file: AppLink) {
    if (select) {
      forDelete.value.push(file)
    } else {
      forDelete.value = forDelete.value.filter((f) => f.id !== file.id)
    }
  }

  /**
   * Add single file to select list
   */
  function selectAll(files: AppLink[], fileId?: string | null) {
    forDelete.value = files.filter((f) => {
      if (fileId && f.file_id !== fileId) {
        return false
      }

      return true
    })
  }

  /**
   * Add or update a new item in the list
   */
  function upsertItem(item: AppLink): void {
    if (hasItem(item.id, item.file_id || null)) {
      updateItem(item)
    } else {
      addItem(item)
    }
  }

  /**
   * Get copy of the item from the list
   */
  function getItem(id: string): AppLink | null {
    const index = items.value.findIndex((item) => item.id === id)

    return items.value[index] || null
  }

  /**
   * Remove item from the list
   */
  function takeItem(id: string): AppLink | null {
    const index = items.value.findIndex((item) => item.id === id)
    return items.value.slice(index, 1)[0] || null
  }

  /**
   * Remove item from the list
   */
  function hasItem(id: string, file_id: string | null): boolean {
    return items.value.findIndex((item) => item.id === id && item.file_id === file_id) !== -1
  }

  /**
   * Update existing item in the list
   */
  function updateItem(file: AppLink) {
    const index = items.value.findIndex((item) => item.id === file.id)

    if (index === -1) {
      return
    }

    items.value.splice(index, 1, file)
  }

  /**
   * Add new item to the list
   */
  function addItem(item: AppLink): void {
    items.value.push(item)
  }

  /**
   * Remove item from the list
   */
  function removeItem(id: string): void {
    items.value = items.value.filter((item) => item.id !== id)
  }

  /**
   * Share a file with a publicly accessible link.
   */
  async function create(file: ListAppFile, kp: KeyPair): Promise<AppLink> {
    const createLink = await meta.createLinkFromFile(file, kp)

    const response = await Api.post<CreateLink, EncryptedAppLink>(
      '/api/links',
      undefined,
      createLink
    )

    if (!response.body) {
      throw new Error('Failed to create link')
    }

    return crypto.decryptLinkRsa(response.body, kp)
  }

  /**
   * Delete the link.
   */
  async function del(id: string): Promise<void> {
    await Api.delete(`/api/links/${id}`)

    removeItem(id)
  }

  /**
   * Mark the link as expired so it cannot be downloaded anymore.
   */
  async function expire(id: string): Promise<void> {
    const expires_at = utcStringFromLocal(new Date())

    await Api.put(`/api/links/${id}`, undefined, {
      expires_at
    })

    const link = takeItem(id)

    if (!link) {
      return
    }

    addItem({ ...link, expires_at })
  }

  /**
   * Load all the shared links for the user.
   */
  async function find(kp: KeyPair): Promise<void> {
    loading.value = true
    const response = await Api.get<EncryptedAppLink[]>(`/api/links`)

    if (!Array.isArray(response.body)) {
      throw new Error('Failed to get link')
    }

    const encryptedLinks = await meta.all()

    const links = await Promise.all(encryptedLinks.map((link) => crypto.decryptLinkRsa(link, kp)))

    for (const link of links) {
      upsertItem(link)
    }

    loading.value = false
  }

  /**
   * Get link from the store (as its owner)
   */
  async function get(id: string, key: string): Promise<AppLink> {
    const link = getItem(id)

    if (link) {
      return link
    }

    return await meta.metadata(id, key)
  }

  return {
    addItem,
    create,
    del,
    expire,
    find,
    get,
    getItem,
    hasItem,
    removeItem,
    selectAll,
    selectOne,
    takeItem,
    updateItem,
    upsertItem,
    forDelete,
    items,
    loading
  }
})
