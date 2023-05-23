import { defineStore } from 'pinia'
import { ref } from 'vue'
import Api from '!/api'
import * as meta from './meta'
import * as crypto from './crypto'
import type { AppLink, CreateLink, EncryptedAppLink, KeyPair, ListAppFile } from 'types'
import { utcStringFromLocal } from '..'

export { meta, crypto }

export const store = defineStore('linkStore', () => {
  const loading = ref(false)

  const items = ref<AppLink[]>([])

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
  async function all(kp: KeyPair): Promise<void> {
    const response = await Api.get<EncryptedAppLink[]>(`/api/links`)

    if (!Array.isArray(response.body)) {
      throw new Error('Failed to get link')
    }

    const encryptedLinks = await meta.all()

    const links = await Promise.all(encryptedLinks.map((link) => crypto.decryptLinkRsa(link, kp)))

    for (const link of links) {
      upsertItem(link)
    }
  }

  return {
    addItem,
    all,
    create,
    del,
    expire,
    getItem,
    hasItem,
    removeItem,
    takeItem,
    updateItem,
    upsertItem,
    items,
    loading
  }
})
