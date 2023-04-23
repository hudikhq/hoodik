import * as meta from '../meta'
import { ErrorResponse } from '../../api'
import { localDateFromUtcString, utcStringFromLocal } from '../..'
import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as sync from './sync'
import { pushChunkToWorker } from '../workers'
import {
  CHUNK_SIZE_BYTES,
  CONCURRENT_CHUNKS_UPLOAD,
  FILES_UPLOADING_AT_ONE_TIME
} from '../constants'

import type {
  CreateFile,
  UploadProgressFunction,
  UploadAppFile,
  IntervalType,
  FilesStore
} from '../types'
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

    const worker = !!('SW' in window)

    return setInterval(async () => {
      if (active.value) {
        await _tick(tracker, worker)
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
    if (error) {
      file.error = error
      file.cancel = true
    }

    const currentFileId = file.file_id || null
    const currentDirId = storage?.dir?.id || null

    if (storage && currentFileId === currentDirId) {
      storage.upsertItem(file)
    }

    console.log(
      `File ${file.metadata?.name} ${file.chunks_stored} / ${file.chunks} has been uploaded`
    )

    /// Get the index of uploading file if it exists
    const index = uploading.value.findIndex((f) => f.id === file.id)

    if (index === -1) {
      console.log(`File ${file.metadata?.name} not found in the uploading list, adding...`)

      // File hasn't been found in the uploading list so we add it
      uploading.value.push(file)
    }

    let item = uploading.value.splice(index, 1)[0]

    // TODO:
    // If the file has been canceled, we will remove it from the uploading list
    // and stop the loop sending the chunks to the worker
    // This is currently not working ... we must handle this somehow
    if (item.cancel) {
      console.log(`File ${file.metadata?.name} is canceling the upload...`)

      item = file
      item.cancel = true
    }

    // If the file has been finished, we will remove it from the uploading list
    // and move it to the done list
    if (isDone || file.finished_upload_at) {
      console.log(`File ${file.metadata?.name} has finished uploading, pushing to the done list...`)

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
  async function _tick(tracker: UploadProgressFunction, worker: boolean) {
    let batch: UploadAppFile[] = []

    if (uploading.value.length < FILES_UPLOADING_AT_ONE_TIME) {
      batch = waiting.value.splice(0, FILES_UPLOADING_AT_ONE_TIME - uploading.value.length)
    }

    return new Promise((resolve) => {
      if (batch.length) {
        // We don't wait for this promise, it will be left to run in the background
        Promise.all(
          batch.map((file) => {
            upload(file, tracker, worker).catch((err) => {
              if (err instanceof ErrorResponse) {
                const error = err as ErrorResponse<unknown>
                setFailed({ ...file, error })
              } else {
                const error = err as Error
                setFailed({ ...file, error })
              }
            })
          })
        )
      }

      done.value = done.value.filter((file) => {
        if (file.finished_upload_at) {
          const date = localDateFromUtcString(file.finished_upload_at).valueOf() + 120 * 1000

          return date < new Date().valueOf()
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
   * Move the file back into the queue for another try at uploading
   */
  function retry(file: UploadAppFile) {
    for (let i = 0; i < failed.value.length; i++) {
      if (failed.value[i].id === file.id) {
        failed.value.splice(i, 1)

        break
      }
    }

    waiting.value.push(file)
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
    retry,
    push,
    create,
    progress
  }
})

/**
 * Upload single file from the upload queue
 */
export async function upload(
  file: UploadAppFile,
  progress?: UploadProgressFunction,
  useWorker?: boolean
) {
  if (!file.started_upload_at) {
    file.started_upload_at = utcStringFromLocal()
  }

  const concurrency = useWorker ? CONCURRENT_CHUNKS_UPLOAD : 1

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

      if (useWorker) {
        await pushChunkToWorker(file, data, chunk)
      } else {
        file = await sync.uploadChunk(file, data, chunk)

        if (progress) {
          const storedChunks = file.uploaded_chunks?.length || 0
          await progress(file, storedChunks === file.chunks)
        }
      }

      return file
    }
  })

  while (workers.length) {
    const batch = workers.splice(0, concurrency)
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
