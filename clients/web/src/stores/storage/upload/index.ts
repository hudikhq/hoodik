import type { AppFile, CreateFile } from '../meta'
import * as meta from '../meta'
import { ErrorResponse } from '../../api'
import { localDateFromUtcString, utcStringFromLocal } from '../..'
import type { KeyPair } from '../../cryptfns/rsa'
import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { store as storageStore } from '../'
import * as http from './http'

export const CHUNK_SIZE_BYTES = 1024 * 1024 * 8
export const MAX_RETRIES = 3
export const UPLOAD_BATCH = 1

export interface SingleChunk {
  data: Uint8Array
  chunk: number
}

export type ProgressFunction = (file: UploadAppFile, done: boolean) => Promise<void>

export interface UploadAppFile extends AppFile {
  /**
   * File system file
   */
  file: File

  /**
   * Start of the upload
   */
  started_upload_at?: Date

  /**
   * Last progress report
   */
  last_progress_at?: Date

  /**
   * Possible error while uploading the file
   */
  error?: ErrorResponse<unknown> | Error | string

  /**
   * Signalize the file to cancel the upload
   */
  cancel?: boolean
}

export const store = defineStore('storage-upload', () => {
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
   * Start processing queue while its not stopped
   */
  async function start(storage?: ReturnType<typeof storageStore>, keypair?: KeyPair) {
    active.value = true

    // Tracker is called each time a file chunk has been uploaded
    const tracker = async function (file: UploadAppFile) {
      const currentFileId = storage?.dir?.id || null

      console.log(
        `File ${file.metadata?.name} ${file.chunks_stored} / ${file.chunks} has been uploaded`
      )

      /// Get the index of uploading file if it exists
      const index = uploading.value.findIndex((f) => f.id === file.id)

      if (index === -1) {
        console.log(`File ${file.metadata?.name} not found in the uploading list, adding...`)

        // File hasn't been found in the uploading list so we add it
        uploading.value.push(file)

        if (storage && keypair && currentFileId === file.file_id) {
          await storage.find(keypair, file.file_id || undefined)
        }
      }

      let item = uploading.value.splice(index, 1)[0]

      // If the file has been canceled, we will remove it from the uploading list
      if (item.cancel) {
        console.log(`File ${file.metadata?.name} is canceling the upload...`)

        item = file
        item.cancel = true

        // Throw an error to stop the upload process
        throw new Error('Upload canceled')
      }

      // If the file has been finished, we will remove it from the uploading list
      // and move it to the done list
      if (file.finished_upload_at) {
        console.log(
          `File ${file.metadata?.name} has finished uploading, pushing to the done list...`
        )

        done.value.push(file)

        if (storage && keypair && currentFileId === file.file_id) {
          await storage.find(keypair, file.file_id || undefined)
        }

        return
      }

      // Update the file in the uploading list
      uploading.value.unshift(file)
    }

    console.log('Starting upload queue')

    while (active.value) {
      await _tick(tracker)
    }
  }

  /**
   * Run single tick of the upload queue that takes the waiting
   * files and starts the upload process for them
   */
  async function _tick(tracker: ProgressFunction) {
    let batch: UploadAppFile[] = []

    if (uploading.value.length < UPLOAD_BATCH) {
      batch = waiting.value.splice(0, UPLOAD_BATCH - uploading.value.length)
    }

    return new Promise((resolve) => {
      if (batch.length) {
        // We don't wait for this promise, it will be left to run in the background
        Promise.all(
          batch.map((file) => {
            http.upload(file, tracker).catch((err) => {
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

      // Wait for 5 seconds before resolving to not spam the
      // application with multiple checks.
      setTimeout(() => {
        done.value = done.value.filter((file) => {
          if (file.finished_upload_at) {
            const date = localDateFromUtcString(file.finished_upload_at).valueOf() + 120 * 1000

            return date < new Date().valueOf()
          }

          return false
        })
        resolve(undefined)
      }, 1000)
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
    create
  }
})
