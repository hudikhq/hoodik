import * as meta from './meta'
import * as queue from './queue'
import * as upload from './upload'
import * as download from './download'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { utcStringFromLocal } from '..'
import type { KeyPair } from '../cryptfns/rsa'
import { FileMetadata } from './metadata'

import type { AppFile, CreateFile, FileResponse, ListAppFile, Parameters } from './types'

export { meta, upload, download, queue }

export const store = defineStore('filesStore', () => {
  /**
   * Are we loading the files?
   */
  const loading = ref(false)

  /**
   * Title of the currently selected directory (or root)
   */
  const title = computed<string>((): string => {
    return dir.value?.metadata?.name || 'Root'
  })

  /**
   * Lookup parameters
   */
  const parameters = computed<Parameters>(() => {
    const parameters: Parameters = {}

    if (dir.value) {
      parameters['dir_id'] = dir.value?.id
    }

    return parameters
  })

  /**
   * Content of the currently selected directory (or root)
   */
  const items = ref<ListAppFile[]>([])

  /**
   * Currently selected directory id
   */
  const fileId = ref<number | null>(null)

  /**
   * Last error message that happened when trying to
   * fetch the files from the backend.
   */
  const error = ref<string | null>(null)

  /**
   * Currently selected directory
   */
  const dir = computed<ListAppFile | null>(() => {
    return items.value.find((item) => item.mime === 'dir' && item.id === fileId.value) || null
  })

  /**
   * All the parent directories
   */
  const parents = computed<ListAppFile[]>(() => {
    const p: ListAppFile[] = []

    const find = (id: number | null) => {
      const i = items.value.find((item) => item.id === id)

      if (i) {
        p.push(i)
      }

      if (i?.file_id) {
        find(i.file_id)
      }
    }

    find(fileId.value)

    return p.reverse()
  })

  /**
   * Head over to backend and do a lookup for the current directory
   */
  async function find(kp: KeyPair, parentId: number | null): Promise<void> {
    loading.value = true
    error.value = null
    fileId.value = parentId

    let query = parameters.value
    if (fileId.value !== undefined && fileId.value !== null) {
      query = { ...parameters.value, dir_id: fileId.value }
    }

    let response: FileResponse = { children: [], parents: [] }

    // We wrap this here so we can somewhat support failing network
    // connection and use the files we have in the store.
    try {
      response = await meta.find(query)
    } catch (e) {
      error.value = `Seems like we are having some kind of problem with getting the files: ${
        (e as Error).message
      }`

      console.warn(error.value)
    }

    response.parents?.forEach(async (item) => {
      upsertItem(await decryptItem({ ...item, parent: true }, kp))
    })

    response.children?.forEach(async (item) => {
      upsertItem(await decryptItem({ ...item, parent: false }, kp))
    })

    loading.value = false
  }

  /**
   * Decrypt each item
   */
  async function decryptItem(item: ListAppFile, kp: KeyPair): Promise<ListAppFile> {
    return {
      ...item,
      metadata: await FileMetadata.decrypt(item.encrypted_metadata, kp),
      encrypted: false
    }
  }

  /**
   * Add or update a new item in the list
   */
  function upsertItem(item: ListAppFile): void {
    if (hasItem(item.id, item.file_id || null)) {
      updateItem(item)
    } else {
      addItem(item)
    }
  }

  /**
   * Remove item from the list
   */
  function hasItem(id: number, file_id: number | null): boolean {
    return items.value.findIndex((item) => item.id === id && item.file_id === file_id) !== -1
  }

  /**
   * Update existing item in the list
   */
  function updateItem(file: ListAppFile) {
    const index = items.value.findIndex((item) => item.id === file.id)

    if (index === -1) {
      return
    }

    items.value[index] = file
  }

  /**
   * Add new item to the list
   */
  function addItem(item: ListAppFile): void {
    items.value.push(item)
  }

  /**
   * Remove item from the list
   */
  function removeItem(id: number): void {
    items.value = items.value.filter((item) => item.id !== id)
  }

  /**
   * Download and decrypt file to the local machine
   */
  async function get(file: ListAppFile): Promise<void> {
    if (!file.id) {
      throw new Error('Cannot download file without ID')
    }

    if (file.mime === 'dir') {
      throw new Error('Cannot download directory')
    }

    if (!file.metadata?.key) {
      throw new Error('Cannot download file without key')
    }

    return download.download(file)
  }

  /**
   * Remove a single file from the storage
   */
  async function remove(kp: KeyPair, file: Partial<ListAppFile>): Promise<void> {
    if (!file.id) {
      throw new Error('Cannot remove file without ID')
    }

    await meta.remove(file.id)
    removeItem(file.id)

    await find(kp, fileId.value)
  }

  /**
   * Create a directory in the storage
   */
  async function createDir(keypair: KeyPair, name: string, dir_id?: number): Promise<AppFile> {
    const createFile: CreateFile = {
      name,
      mime: 'dir',
      file_id: dir_id,
      file_created_at: utcStringFromLocal(new Date())
    }

    const created = await meta.create(keypair, createFile)

    return { ...created }
  }

  return {
    dir,
    parents,
    loading,
    title,
    items,
    parameters,
    get,
    find,
    remove,
    createDir,
    hasItem,
    updateItem,
    upsertItem,
    addItem,
    removeItem
  }
})
