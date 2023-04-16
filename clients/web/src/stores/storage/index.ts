import * as meta from './meta'
import * as queue from './queue'
import * as upload from './upload'
import * as download from './download'
import type { KeyPair } from '../cryptfns/rsa'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'

export { meta, upload, download, queue }

export interface ListAppFile extends Partial<meta.AppFile> {
  current?: boolean
  parent?: boolean
  encrypted?: boolean
}

/**
 * Decrypt file metadata
 */
export async function decrypt(
  file: meta.AppFile,
  kp: KeyPair,
  progress?: (id?: number) => void
): Promise<meta.AppFile> {
  file = {
    metadata: await meta.FileMetadata.decrypt(file.encrypted_metadata, kp),
    ...file
  }

  if (progress) {
    progress(file.id)
  }

  return file
}

/**
 * Format bytes to human readable string
 */
export function format(b?: number | string): string {
  if (b === undefined || b === null) {
    return '0 B'
  }

  if (typeof b === 'string') {
    b = parseInt(b)
  }

  const kb = b / 1024

  if (kb < 2048) {
    return `${kb.toFixed(2)} MB`
  }

  const mb = b / 1024 / 1024

  if (mb < 2048) {
    return `${mb.toFixed(2)} MB`
  }

  const gb = b / 1024 / 1024 / 1024

  return `${gb.toFixed(2)} GB`
}

export const store = defineStore('storage', () => {
  /**
   * Are we loading the files?
   */
  const loading = ref(false)

  /**
   * Parent directory of the currently selected one
   */
  const parent = ref<meta.AppFile | null>(null)

  /**
   * Currently selected directory
   */
  const dir = ref<meta.AppFile | null>(null)

  /**
   * Title of the currently selected directory (or root)
   */
  const title = computed<string>((): string => {
    return dir.value?.metadata?.name || 'Root'
  })

  /**
   * Lookup parameters
   */
  const parameters = computed<meta.Parameters>(() => {
    const parameters: meta.Parameters = {}

    if (dir.value) {
      parameters['file_id'] = dir.value.id
    }

    return parameters
  })

  /**
   * Content of the currently selected directory (or root)
   */
  const items = ref<ListAppFile[]>([])

  /**
   * Head over to backend and do a lookup for the current directory
   */
  async function find(kp: KeyPair): Promise<void> {
    loading.value = true

    const response = await meta.find(parameters.value)

    let results: ListAppFile[] = response.children.map((item) => ({ ...item, encrypted: true }))

    if (response.dir) {
      dir.value = response.dir
      results.unshift({ ...response.dir, current: true, encrypted: true, mime: 'dir' })
    } else {
      dir.value = null
    }

    if (response.parent) {
      parent.value = response.parent
      results.unshift({ ...response.parent, parent: true, encrypted: true, mime: 'dir' })
    } else {
      parent.value = null
    }

    // Decrypt all the files names and keys
    results = await Promise.all(
      results.map(async (item): Promise<ListAppFile> => {
        if (!item.id || !item.encrypted_metadata) {
          return item
        }

        return decrypt(item as meta.AppFile, kp)
      })
    )

    items.value = results
    loading.value = false
  }

  /**
   * Remove a single file from the storage
   */
  async function remove(kp: KeyPair, file: Partial<ListAppFile>): Promise<void> {
    if (!file.id) {
      throw new Error('Cannot remove file without ID')
    }

    await meta.remove(file.id)
    await find(kp)
  }

  return {
    dir,
    parent,
    loading,
    title,
    items,
    parameters,
    find,
    remove
  }
})
