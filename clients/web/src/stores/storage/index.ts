import * as meta from './meta'
import * as queue from './queue'
import * as upload from './upload'
import * as download from './download'

import * as cryptfns from '../cryptfns'
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { UploadAppFile } from './upload'
import { utcStringFromLocal } from '..'
import type { CreateFile } from './meta'

export { meta, upload, download, queue }

export interface ListAppFile extends meta.AppFile {
  current?: boolean
  parent?: boolean
  encrypted?: boolean
  name?: string
}

/**
 * Decrypt file metadata
 */
export async function decrypt(
  file: meta.AppFile,
  kp: cryptfns.rsa.KeyPair,
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

  if (b < 2048) {
    return `${b.toFixed(2)} B`
  }

  const kb = b / 1024

  if (kb < 2048) {
    return `${kb.toFixed(2)} KB`
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
   * All the parent directories
   */
  const parents = ref<meta.AppFile[]>([])

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
      parameters['dir_id'] = dir.value.id
    }

    return parameters
  })

  /**
   * Content of the currently selected directory (or root)
   */
  const items = ref<ListAppFile[]>([])

  const _order = ref<{
    key: keyof ListAppFile
    direction: 'asc' | 'desc'
  }>({ key: 'finished_upload_at', direction: 'desc' })

  /**
   * Order the list of files
   */
  function order(key?: keyof ListAppFile, direction?: 'asc' | 'desc') {
    _order.value = { key: key || _order.value.key, direction: direction || _order.value.direction }

    items.value.sort((a, b) => {
      if (a.mime === 'dir') {
        return -1
      }

      // @ts-ignore
      if (key === 'name') {
        const aName = a.metadata?.name || ''
        const bName = b.metadata?.name || ''
        if (_order.value.direction === 'asc') {
          return aName < bName ? -1 : 1
        } else {
          return aName > bName ? -1 : 1
        }
      }

      if (a[_order.value.key] === undefined) {
        return -1
      }
      if (b[_order.value.key] === undefined) {
        return 0
      }

      if (a[_order.value.key] === b[_order.value.key]) {
        return 0
      }

      if (_order.value.direction === 'asc') {
        // @ts-ignore
        return a[_order.value.key] < b[_order.value.key] ? -1 : 1
      } else {
        // @ts-ignore
        return a[_order.value.key] > b[_order.value.key] ? -1 : 1
      }
    })
  }

  /**
   * Head over to backend and do a lookup for the current directory
   */
  async function find(kp: cryptfns.rsa.KeyPair, dir_id?: number | null): Promise<void> {
    loading.value = true

    let query = parameters.value

    if (dir_id !== undefined && dir_id !== null) {
      query = { ...parameters.value, dir_id }
    }

    const response = await meta.find(query)

    let results: ListAppFile[] = response.children.map((item) => ({ ...item, encrypted: true }))

    response.parents?.forEach((item) => {
      results.push({ ...item, parent: true })
    })

    // Decrypt all the files names and keys
    results = await Promise.all(
      results.map(async (item): Promise<ListAppFile> => {
        if (!item.id || !item.encrypted_metadata) {
          return item
        }

        return decrypt(item as meta.AppFile, kp)
      })
    )

    parents.value = results.slice(response.children.length)
    items.value = results.slice(0, response.children.length)
    dir.value = parents.value[parents.value.length - 1] || null

    order()

    loading.value = false
  }

  /**
   * Download and decrypt file to the local machine
   */
  async function get(kp: cryptfns.rsa.KeyPair, file: ListAppFile): Promise<void> {
    if (!file.id) {
      throw new Error('Cannot download file without ID')
    }

    if (file.mime === 'dir') {
      throw new Error('Cannot download directory')
    }

    if (!file.metadata?.key) {
      throw new Error('Cannot download file without key')
    }

    const { body } = await download.getResponse(file.id)

    if (!body) {
      throw new Error('File cannot be downloaded, missing body from response')
    }

    let i = 0
    const transformStream = new TransformStream({
      transform(chunk, controller) {
        const decryptedChunk = cryptfns.aes.decrypt(chunk, file.metadata?.key as cryptfns.aes.Key)

        i += decryptedChunk.length

        console.log(
          'chunk size',
          format(chunk.byteLength),
          'Total decrypted',
          format(i),
          'Key size',
          file.metadata?.key?.blocksize
        )

        controller.enqueue(decryptedChunk)
      }
    })

    const decryptedStream = body.pipeThrough(transformStream)

    const filename = file.metadata?.name || file.id.toString()

    const init = {
      headers: new Headers({
        'Content-Disposition': `attachment; filename="${filename}"`,
        'Content-Type': file.mime
      })
    }
    const response = new Response(decryptedStream, init)

    // Trigger the download using the Download API
    const anchor = document.createElement('a')
    anchor.href = URL.createObjectURL(await response.blob())
    anchor.download = filename
    document.body.appendChild(anchor)
    anchor.click()
    document.body.removeChild(anchor)
  }

  /**
   * Push file to the storage (used for uploads)
   */
  async function push(file: UploadAppFile): Promise<void> {
    const index = items.value.findIndex((item) => item.id === file.id)

    if (index !== -1) {
      items.value.splice(index, 1)
    }

    items.value.push(file)
    order()
  }

  /**
   * Remove a single file from the storage
   */
  async function remove(kp: cryptfns.rsa.KeyPair, file: Partial<ListAppFile>): Promise<void> {
    if (!file.id) {
      throw new Error('Cannot remove file without ID')
    }

    await meta.remove(file.id)
    await find(kp)
  }

  /**
   * Create a directory in the storage
   */
  async function createDir(
    keypair: cryptfns.rsa.KeyPair,
    name: string,
    dir_id?: number
  ): Promise<meta.AppFile> {
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
    order,
    push,
    createDir
  }
})
