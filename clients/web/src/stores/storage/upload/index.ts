import * as meta from '../meta'
import { ErrorResponse } from '../../api'
import { errorIntoWorkerError, localDateFromUtcString, utcStringFromLocal } from '../..'
import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as sync from './sync'
import { pushUploadToWorker } from '../workers'
import {
  CHUNK_SIZE_BYTES,
  FILES_UPLOADING_AT_ONE_TIME,
  KEEP_FINISHED_UPLOADS_FOR_MINUTES
} from '../constants'

import type {
  CreateFile,
  UploadProgressFunction,
  UploadAppFile,
  IntervalType,
  FilesStore
} from '../../types'
import type { store as filesStore } from '../'
import type { KeyPair } from '../../cryptfns/rsa'

export const store = defineStore('upload', () => {
  /**
   * Start processing queue while its not stopped
   */
  async function start(storage: ReturnType<typeof filesStore>): Promise<IntervalType> {
    active.value = true

    console.log('Starting upload queue')

    const tracker = (file: UploadAppFile, isDone: boolean) => progress(storage, file, isDone)

    return setInterval(async () => {
      if (active.value) {
        await _tick(tracker)
      }
    }, 1000)
  }

  /**
   * Files ready for upload and files currently being uploaded
   */
  const waiting = ref<UploadAppFile[]>([])

  /**
   * Files currently being uploaded
   */
  const uploading = ref<UploadAppFile[]>([])

  /**
   * Files that failed the uploading process
   */
  const failed = ref<UploadAppFile[]>([])

  /**
   * Files that finished the uploading process
   */
  const done = ref<UploadAppFile[]>([])

  /**
   * Is the queue currently being processed
   */
  const active = ref(false)

  /**
   * Create function that will track the progress
   */
  async function progress(storage: FilesStore, file: UploadAppFile, isDone: boolean, error?: any) {
    if (done.value.filter((f) => f.id === file.id).length !== 0) {
      return
    }

    // Remove it from the uploading list
    uploading.value = uploading.value.filter((f) => f.id !== file.id)

    if (error) {
      file.error = error
      file.cancel = true
    }

    // If it already exists in the failed list, we don't want to
    if (file.cancel && failed.value.filter((f) => f.id === file.id).length !== 0) {
      return
    }

    const currentFileId = file.file_id || null
    const currentDirId = storage?.dir?.id || null

    // Upsert the item in the storage
    if (!file.cancel && storage && currentFileId === currentDirId) {
      storage.upsertItem(file)
    }

    // Canceling the upload is done by deleting the file on the server,
    // that will trigger the upload error and the file will be moved to the
    // failed list as if it was canceled
    if (file.cancel) {
      console.log(`File ${file.metadata?.name} is canceling the upload...`)

      failed.value.push(file)
      uploading.value = uploading.value.filter((i) => i.id !== file.id)
      storage.removeItem(file.id)

      return
    }

    // If the file has been finished, we will remove it from the uploading list
    // and move it to the done list
    if (isDone || file.finished_upload_at) {
      console.log(`File ${file.metadata?.name} has finished uploading, pushing to the done list...`)

      file.finished_upload_at = utcStringFromLocal(new Date())
      done.value.push(file)

      return
    }

    // Update the file in the uploading list
    uploading.value.unshift(file)
  }

  /**
   * Run single tick of the upload queue that takes the waiting
   * files and starts the upload process for them
   */
  async function _tick(tracker: UploadProgressFunction) {
    let batch: UploadAppFile[] = []

    if (uploading.value.length < FILES_UPLOADING_AT_ONE_TIME) {
      batch = waiting.value.splice(0, FILES_UPLOADING_AT_ONE_TIME - uploading.value.length)
    }

    return new Promise((resolve) => {
      if (batch.length) {
        // We don't wait for this promise, it will be left to run in the background
        Promise.all(
          batch.map((file) => {
            const promise = 'SW' in window ? pushUploadToWorker(file) : upload(file, tracker)

            promise.catch((err) => {
              setFailed({ ...file, error: errorIntoWorkerError(err) })
            })
          })
        )
      }

      done.value = done.value.filter((file) => {
        if (file.finished_upload_at) {
          const date =
            localDateFromUtcString(file.finished_upload_at).valueOf() +
            KEEP_FINISHED_UPLOADS_FOR_MINUTES * 60 * 1000

          return new Date().valueOf() < date
        }

        return false
      })

      resolve(undefined)
    })
  }

  /**
   * Set a file in failed state
   */
  function setFailed(file: UploadAppFile) {
    for (let i = 0; i < uploading.value.length; i++) {
      if (uploading.value[i].id === file.id) {
        uploading.value.splice(i, 1)

        break
      }
    }

    failed.value.push(file)
  }

  /**
   * Add new file to the upload queue
   */
  async function push(keypair: KeyPair, file: File, parent_id?: number) {
    try {
      const existing = await meta.getByName(keypair, file.name, parent_id)

      return waiting.value.push({ ...existing, file })
    } catch (e) {
      if (!(e instanceof ErrorResponse) || e.status !== 404) {
        throw e
      }

      const created = await create(keypair, file, parent_id)

      return waiting.value.push(created)
    }
  }

  /**
   * Cancel the upload of a file
   */
  async function cancel(files: FilesStore, file: UploadAppFile) {
    if (uploading.value.filter((f) => f.id === file.id).length === 0) {
      throw new Error('File cannot be canceled when its not uploading')
    }

    file.cancel = true

    await meta.remove(file.id)
    files.removeItem(file.id)

    await progress(files, file, false, new Error('Upload canceled'))
  }

  /**
   * Create new file metadata and add it to the upload queue
   */
  async function create(keypair: KeyPair, file: File, parent_id?: number): Promise<UploadAppFile> {
    const modified = file.lastModified ? new Date(file.lastModified) : new Date()

    const createFile: CreateFile = {
      name: file.name,
      size: file.size,
      mime: file.type || 'application/octet-stream',
      chunks: Math.ceil(file.size / CHUNK_SIZE_BYTES),
      file_id: parent_id,
      file_created_at: utcStringFromLocal(modified)
    }

    const created = await meta.create(keypair, createFile)

    return { ...created, file }
  }

  return {
    waiting,
    uploading,
    failed,
    done,
    active,
    start,
    push,
    create,
    cancel,
    progress
  }
})

/**
 * Upload single file from the upload queue
 */
export async function upload(file: UploadAppFile, progress?: UploadProgressFunction) {
  if (!file.started_upload_at) {
    file.started_upload_at = utcStringFromLocal()
  }

  if (progress) {
    await progress(file, false)
  }

  const workers = [...new Array(file.chunks)].map((_, chunk) => {
    return async () => {
      // Skip already uploaded chunks
      if (file.uploaded_chunks?.includes(chunk)) {
        if (progress) {
          const storedChunks = file.uploaded_chunks?.length || 0
          await progress(file, storedChunks === file.chunks)
        }

        return file
      }

      const data = await sliceChunk(file.file as File, chunk)

      file = await sync.uploadChunk(file, data, chunk)

      if (progress) {
        const storedChunks = file.uploaded_chunks?.length || 0
        await progress(file, storedChunks === file.chunks)
      }

      return file
    }
  })

  while (workers.length) {
    const batch = workers.splice(0, 1)
    file = await Promise.race(batch.map((worker) => worker()))
  }

  return file
}

/**
 * Perform slicing of the file chunk with a fallback in case
 * the browser does not support the arrayBuffer method on the Blob
 */
async function sliceChunk(file: File, chunk: number): Promise<Uint8Array> {
  const start = chunk * CHUNK_SIZE_BYTES
  const end = (chunk + 1) * CHUNK_SIZE_BYTES

  const slice = file.slice(start, end)

  if (typeof slice.arrayBuffer === 'function') {
    return new Uint8Array(await slice.arrayBuffer())
  }

  return new Promise((resolve, reject) => {
    const reader = new FileReader()

    reader.onload = () => {
      if (reader.result instanceof ArrayBuffer) {
        resolve(new Uint8Array(reader.result))
      } else {
        reject(new Error('Failed to read file'))
      }
    }

    reader.onerror = (err) => {
      reject(err)
    }

    reader.readAsArrayBuffer(slice)
  })
}
