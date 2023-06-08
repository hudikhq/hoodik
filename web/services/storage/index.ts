import * as meta from './meta'
import * as queue from '../queue'
import * as upload from './upload'
import * as download from './download'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import * as cryptfns from '../cryptfns'
import { utcStringFromLocal, uuidv4 } from '..'
import { useStorage } from '@vueuse/core'

import type {
  AppFile,
  CreateFile,
  FileResponse,
  Parameters,
  EncryptedAppFile,
  KeyPair
} from 'types'

export { meta, upload, download, queue }

/**
 * Run sort operations on the given items by the given parameter
 * The results are always in ASC, if you need DESC, just reverse the array
 */
function innerSort(items: AppFile[], parameter: string): AppFile[] {
  return items.sort((a, b) => {
    // @ts-ignore
    const aValue = a[parameter] || ''
    // @ts-ignore
    const bValue = b[parameter] || ''

    if (typeof aValue === 'number' && typeof bValue === 'number') {
      return aValue - bValue
    }

    return aValue.localeCompare(bValue)
  })
}

export const store = defineStore('files', () => {
  /**
   * Are we loading the files?
   */
  const loading = ref(false)

  /**
   * Title of the currently selected directory (or root)
   */
  const title = computed<string>((): string => {
    return dir.value?.name || 'Root'
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
  const _items = ref<AppFile[]>([])

  /**
   * Persistent storage of the sort options
   */
  const sort = useStorage<{ [key: string]: string }>('dir-sort', {})

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
   * Files selected to be deleted from various places
   */
  const forDelete = ref<AppFile[]>([])

  /**
   * Current dir sort options
   */
  const sortOptions = computed<{ parameter: string; order: string }>(() => {
    const [parameter, order] = getSort(fileId.value || 'root').split('|')

    return {
      parameter,
      order
    }
  })

  /**
   * Currently selected directory
   */
  const dir = computed<AppFile | null>(() => {
    return _items.value.find((item) => item.mime === 'dir' && item.id === fileId.value) || null
  })

  /**
   * All the parent directories
   */
  const parents = computed<AppFile[]>(() => {
    const p: AppFile[] = []

    const f = (id: string | null) => {
      const i = _items.value.find((item) => item.id === id)

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
   * Items filtered for the given directory and sorted by the sort options
   */
  const items = computed<AppFile[]>(() => {
    const [parameter, order] = getSort(fileId.value || 'root').split('|')

    const directories = innerSort(
      _items.value.filter((item) => {
        if (item.mime !== 'dir') {
          return false
        }

        if (fileId.value) {
          return item.file_id === fileId.value
        }

        return item.file_id === null
      }),
      parameter
    )

    const files = innerSort(
      _items.value.filter((item) => {
        if (item.mime === 'dir') {
          return false
        }

        if (fileId.value) {
          return item.file_id === fileId.value
        }

        return item.file_id === null
      }),
      parameter
    )

    if (order === 'desc') {
      return [...directories.reverse(), ...files.reverse()]
    }

    return [...directories, ...files]
  })

  /**
   * Head over to backend and do a lookup for the current directory
   */
  async function find(kp: KeyPair, parentId: string | null, showLoading = true): Promise<void> {
    error.value = null

    let query = parameters.value
    if (parentId !== undefined && parentId !== null) {
      query = { ...parameters.value, dir_id: parentId }
    } else {
      delete query.dir_id
    }

    fileId.value = parentId

    let response: FileResponse = { children: [], parents: [] }
    loading.value = showLoading

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
      await replaceItem(item, kp)
    })

    response.children?.forEach(async (item) => {
      await replaceItem(item, kp)
    })

    loading.value = false
  }

  /**
   * Attempts to avoid decrypting of an item that is already in the list
   */
  async function replaceItem(item: AppFile, kp: KeyPair): Promise<void> {
    const existing = getItem(item.id)

    if (existing && existing.key) {
      return upsertItem({
        ...item,
        key: existing.key,
        name: existing.name,
        thumbnail: existing.thumbnail,
        temporaryId: uuidv4()
      })
    } else {
      return upsertItem(await decryptItem({ ...item, temporaryId: uuidv4() }, kp))
    }
  }

  /**
   * Decrypt each item
   */
  async function decryptItem(item: AppFile, kp: KeyPair): Promise<AppFile> {
    const decryptedParts = await meta.decrypt(item, kp.input as string)

    return {
      ...item,
      ...decryptedParts
    }
  }

  /**
   * Add or update a new item in the list
   */
  function upsertItem(item: AppFile): void {
    if (hasItem(item.id, item.file_id || null)) {
      updateItem(item)
    } else {
      addItem({ ...item, temporaryId: uuidv4() })
    }
  }

  /**
   * Get copy of the item from the list
   */
  function getItem(id: string): AppFile | null {
    const index = _items.value.findIndex((item) => item.id === id)

    return _items.value[index] || null
  }

  /**
   * Remove item from the list
   */
  function takeItem(id: string): AppFile | null {
    const index = _items.value.findIndex((item) => item.id === id)
    return _items.value.slice(index, 1)[0] || null
  }

  /**
   * Remove item from the list
   */
  function hasItem(id: string, file_id: string | null): boolean {
    return _items.value.findIndex((item) => item.id === id && item.file_id === file_id) !== -1
  }

  /**
   * Update existing item in the list
   */
  function updateItem(file: AppFile) {
    const index = _items.value.findIndex((item) => item.id === file.id)

    if (index === -1) {
      return
    }

    _items.value.splice(index, 1, file)
  }

  /**
   * Add new item to the list
   */
  function addItem(item: AppFile): void {
    _items.value.push(item)
  }

  /**
   * Remove item from the list
   */
  function removeItem(id: string): void {
    _items.value = _items.value.filter((item) => item.id !== id)
    forDelete.value = forDelete.value.filter((item) => item.id !== id)
  }

  /**
   * Download and decrypt file to the local machine
   */
  async function get(file: AppFile, kp: KeyPair): Promise<AppFile> {
    if (!file.id) {
      throw new Error('Cannot download file without ID')
    }

    if (file.mime === 'dir') {
      throw new Error('Cannot download directory')
    }

    if (!file.key) {
      throw new Error('Cannot download file without key')
    }

    return download.get(file, kp)
  }

  /**
   * Load file metadata, use the inner storage if the file is found, if not, fetch it from the backend
   */
  async function metadata(id: string, kp: KeyPair): Promise<AppFile> {
    const item = getItem(id)

    if (!item) {
      const item = await meta.get(kp, id)

      addItem(item)
    }

    return item as AppFile
  }

  /**
   * Remove a single file from the storage
   */
  async function remove(kp: KeyPair, file: Partial<AppFile>): Promise<void> {
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

    return meta.create(keypair, createFile)
  }

  /**
   * Add single file to select list
   */
  function selectOne(select: boolean, file: AppFile) {
    if (select) {
      forDelete.value.push(file)
    } else {
      forDelete.value = forDelete.value.filter((f) => f.id !== file.id)
    }
  }

  /**
   * Add single file to select list
   */
  function selectAll(files: AppFile[], fileId?: string | null) {
    forDelete.value = files.filter((f) => {
      if (fileId && f.file_id !== fileId) {
        return false
      }

      return true
    })
  }

  /**
   * Add single file to select list
   */
  function deselectAll() {
    forDelete.value = []
  }

  /**
   * Remove all the files on the selected list
   */
  async function removeAll(kp: KeyPair, files: AppFile[]) {
    await Promise.all(
      files.map(async (file) => {
        await meta.remove(file.id)
        removeItem(file.id)
      })
    )

    forDelete.value = []

    await find(kp, fileId.value)
  }

  /**
   * Set the sort value for a given directory
   */
  function setSort(dir: string, parameter: string, order: 'asc' | 'desc'): void {
    sort.value[dir] = `${parameter}|${order}`
  }

  /**
   * Simple version of sort that can be used in the UI
   */
  function setSortSimple(value: string): void {
    const [parameter, order] = value.split('|')

    setSort(fileId.value || 'root', parameter, order as 'asc' | 'desc')
  }

  /**
   * Get the sort value for given directory
   */
  function getSort(dir: string): string {
    return sort.value[dir] || 'name|desc'
  }

  return {
    dir,
    parents,
    loading,
    title,
    items,
    parameters,
    forDelete,
    sortOptions,
    addItem,
    createDir,
    decryptItem,
    setSortSimple,
    find,
    get,
    getItem,
    hasItem,
    metadata,
    remove,
    removeAll,
    removeItem,
    replaceItem,
    selectAll,
    deselectAll,
    selectOne,
    takeItem,
    updateItem,
    upsertItem,
    setSort,
    getSort
  }
})

/**
 * Do a full text search through the files and folders
 */
export async function search(query: string, kp: KeyPair): Promise<AppFile[]> {
  if (!kp.input) {
    throw new Error('Cannot search without private key')
  }

  const privateKey = kp.input
  const response = await meta.search(query)

  const results = await Promise.all(
    response.map(async (file: EncryptedAppFile) => {
      const unencryptedPart = await meta.decrypt(file, privateKey)

      return {
        ...file,
        ...unencryptedPart
      }
    })
  )

  return results
}
