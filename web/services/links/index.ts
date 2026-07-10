import { defineStore } from 'pinia'
import { ref } from 'vue'
import Api from '!/api'
import * as logger from '!/logger'
import * as meta from './meta'
import * as crypto from './crypto'
import type { AppLink, CreateLink, EncryptedAppLink, KeyPair, AppFile } from 'types'

export { meta, crypto }

export const store = defineStore('links', () => {
  const loading = ref(false)

  const items = ref<AppLink[]>([])

  /**
   * Files selected to be deleted from various places
   */
  const selected = ref<AppLink[]>([])

  /**
   * Add single link to select list
   */
  function selectOne(select: boolean, link: AppLink) {
    if (select) {
      selected.value.push(link)
    } else {
      selected.value = selected.value.filter((f) => f.id !== link.id)
    }
  }

  /**
   * Select all the links
   */
  function selectAll(links: AppLink[]) {
    selected.value = links
  }

  /**
   * Remove all the selected links
   */
  function deselectAll() {
    selected.value = []
  }

  /**
   * Add or update a new item in the list
   */
  function upsertItem(item: AppLink): void {
    if (hasItem(item.id)) {
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
    const item = items.value[index] || null

    if (item) {
      items.value = items.value.filter((item) => item.id !== id)
    }

    return item
  }

  /**
   * Remove item from the list
   */
  function hasItem(id: string): boolean {
    return items.value.findIndex((item) => item.id === id) !== -1
  }

  /**
   * Update existing item in the list
   */
  function updateItem(link: AppLink) {
    const index = items.value.findIndex((item) => item.id === link.id)

    if (index === -1) {
      return
    }

    items.value.splice(index, 1, link)
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
   * Share a link with a publicly accessible link.
   */
  async function create(link: AppFile, kp: KeyPair): Promise<AppLink> {
    const createLink = await meta.createLinkFromFile(link, kp)

    const response = await Api.post<CreateLink, EncryptedAppLink>(
      '/api/links',
      undefined,
      createLink
    )

    if (!response.body) {
      throw new Error('Failed to create link')
    }

    return crypto.decryptOwnLink(response.body, kp)
  }

  /**
   * Delete the link.
   */
  async function remove(id: string): Promise<void> {
    await Api.delete(`/api/links/${id}`)

    removeItem(id)
  }

  /**
   * Remove all the links on the selected list
   */
  async function removeAll(kp: KeyPair, links: AppLink[]) {
    await Promise.all(
      links.map(async (link) => {
        await Api.delete(`/api/links/${link.id}`)
        removeItem(link.id)
      })
    )

    items.value = []
    selected.value = []

    await find(kp)
  }

  /**
   * Set the expiry on a link
   */
  async function expire(id: string, expiresAt?: Date): Promise<AppLink> {
    let expires_at

    if (expiresAt) {
      expires_at = Math.floor(expiresAt.valueOf() / 1000)
    }

    const link = takeItem(id)

    if (!link) {
      throw new Error('Failed to update link')
    }

    await Api.put(`/api/links/${id}`, undefined, {
      expires_at
    })

    addItem({ ...link, expires_at })

    return { ...link, expires_at }
  }

  /**
   * Load all the public links the caller owns. The endpoint is owner-
   * scoped — public links are entirely owner-side.
   */
  async function find(kp: KeyPair): Promise<void> {
    loading.value = true

    const encryptedLinks = await meta.all()

    // Decrypt per link and drop the ones that fail rather than rejecting the
    // whole list: a single link the current key can't unwrap (e.g. wrapped
    // under a superseded key) must not blank the page for every other link.
    const links = await Promise.all(
      encryptedLinks.map(async (link) => {
        try {
          return await crypto.decryptOwnLink(link, kp)
        } catch (err) {
          logger.warn(`[links] omitting undecryptable link ${link.id}`, err)
          return null
        }
      })
    )

    for (const link of links) {
      if (link) upsertItem(link)
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

    const metadata = await meta.metadata(id, key)

    addItem(metadata)

    return metadata
  }

  /**
   * Fetch and decrypt the link content client-side, per chunk. The link key
   * comes from the URL fragment and never leaves the browser; the server only
   * ever streams ciphertext.
   */
  async function download(id: string, key: string): Promise<Uint8Array> {
    const link = await get(id, key)
    return await meta.downloadAndDecrypt(link)
  }

  /**
   * Decrypt the link content client-side and save it under the real filename
   * from the decrypted link metadata.
   */
  async function formDownload(id: string, key: string): Promise<void> {
    const link = await get(id, key)
    await meta.saveDecrypted(link)
  }

  return {
    addItem,
    create,
    download,
    expire,
    find,
    formDownload,
    get,
    getItem,
    hasItem,
    remove,
    removeAll,
    removeItem,
    selectAll,
    selectOne,
    takeItem,
    updateItem,
    upsertItem,
    deselectAll,
    selected,
    items,
    loading
  }
})
