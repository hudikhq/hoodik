import * as meta from './meta'
import * as queue from '../queue'
import * as upload from './upload'
import * as download from './download'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import * as cryptfns from '../cryptfns'
import { utcStringFromLocal, uuidv4 } from '..'
import { FileMetadata } from './metadata'

import type {
  AppFile,
  CreateFile,
  FileResponse,
  ListAppFile,
  Parameters,
  EncryptedAppFile,
  KeyPair
} from 'types'

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
   * All the items regardless of the current directory
   */
  const items = ref<ListAppFile[]>([])

  /**
   * Currently selected directory id
   */
  const fileId = ref<string | null>(null)

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

    const f = (id: string | null) => {
      const i = items.value.find((item) => item.id === id)

      if (i) {
        p.push(i)
      }

      if (i?.file_id) {
        f(i.file_id)
      }
    }

    f(fileId.value)

    return p.reverse()
  })

  /**
   * Head over to backend and do a lookup for the current directory
   */
  async function find(kp: KeyPair, parentId: string | null): Promise<void> {
    loading.value = true
    error.value = null

    let query = parameters.value
    if (parentId !== undefined && parentId !== null) {
      query = { ...parameters.value, dir_id: parentId }
    } else {
      delete query.dir_id
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
    }

    response.parents?.forEach(async (item) => {
      await replaceItem({ ...item, parent: true }, kp)
    })

    response.children?.forEach(async (item) => {
      await replaceItem({ ...item, parent: false }, kp)
    })

    fileId.value = parentId
    loading.value = false
  }

  /**
   * Attempts to avoid decrypting of an item that is already in the list
   */
  async function replaceItem(item: ListAppFile, kp: KeyPair): Promise<void> {
    const existing = getItem(item.id)

    if (existing) {
      return upsertItem({ ...item, metadata: existing.metadata, temporaryId: uuidv4() })
    } else {
      return upsertItem(await decryptItem({ ...item, temporaryId: uuidv4() }, kp))
    }
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
      addItem({ ...item, temporaryId: uuidv4() })
    }
  }

  /**
   * Get copy of the item from the list
   */
  function getItem(id: string): ListAppFile | null {
    const index = items.value.findIndex((item) => item.id === id)

    return items.value[index] || null
  }

  /**
   * Remove item from the list
   */
  function takeItem(id: string): ListAppFile | null {
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
  function updateItem(file: ListAppFile) {
    const index = items.value.findIndex((item) => item.id === file.id)

    if (index === -1) {
      return
    }

    items.value.splice(index, 1, file)
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
  function removeItem(id: string): void {
    items.value = items.value.filter((item) => item.id !== id)
    forDelete.value = forDelete.value.filter((item) => item.id !== id)
  }

  /**
   * Download and decrypt file to the local machine
   */
  async function get(file: ListAppFile, kp: KeyPair): Promise<ListAppFile> {
    if (!file.id) {
      throw new Error('Cannot download file without ID')
    }

    if (file.mime === 'dir') {
      throw new Error('Cannot download directory')
    }

    if (!file.metadata?.key) {
      throw new Error('Cannot download file without key')
    }

    return download.get(file, kp)
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
  async function createDir(keypair: KeyPair, name: string, dir_id?: string): Promise<AppFile> {
    const search_tokens_hashed = cryptfns.stringToHashedTokens(name.toLowerCase())

    const createFile: CreateFile = {
      name,
      mime: 'dir',
      file_id: dir_id,
      file_created_at: utcStringFromLocal(new Date()),
      search_tokens_hashed
    }

    const created = await meta.create(keypair, createFile)

    return { ...created }
  }

  /**
   * Files selected to be deleted from various places
   */
  const forDelete = ref<AppFile[]>([])

  /**
   * Add single file to select list
   */
  function selectOne(select: boolean, file: ListAppFile) {
    if (select) {
      forDelete.value.push(file)
    } else {
      forDelete.value = forDelete.value.filter((f) => f.id !== file.id)
    }
  }

  /**
   * Add single file to select list
   */
  function selectAll(files: ListAppFile[], fileId?: string | null) {
    forDelete.value = files.filter((f) => {
      if (fileId && f.file_id !== fileId) {
        return false
      }

      return true
    })
  }

  /**
   * Remove all the files on the selected list
   */
  async function removeAll(kp: KeyPair, files: ListAppFile[]) {
    await Promise.all(
      files.map(async (file) => {
        await meta.remove(file.id)
        removeItem(file.id)
      })
    )

    forDelete.value = []

    await find(kp, fileId.value)
  }

  return {
    dir,
    parents,
    loading,
    title,
    items,
    parameters,
    forDelete,
    selectOne,
    selectAll,
    removeAll,
    decryptItem,
    get,
    find,
    remove,
    createDir,
    hasItem,
    getItem,
    takeItem,
    replaceItem,
    updateItem,
    upsertItem,
    addItem,
    removeItem
  }
})

export async function search(query: string, kp: KeyPair): Promise<ListAppFile[]> {
  const results = await Promise.all(
    (
      await meta.search(query)
    ).map(async (file: EncryptedAppFile) => {
      return {
        ...file,
        metadata: await FileMetadata.decrypt(file.encrypted_metadata, kp),
        encrypted: false
      }
    })
  )

  console.log(results)

  return results
}
