import type { AppFile, CreateFile } from './meta'
import * as meta from './meta'
import Api, { ErrorResponse, type Query } from '../api'
import * as cryptfns from '../cryptfns'
import { localDateFromUtcString, utcStringFromLocal } from '..'
import type { KeyPair } from '../cryptfns/rsa'
import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { store as storageStore } from './'

export const CHUNK_SIZE_BYTES = 1024 * 1024 * 10
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
      console.log(
        `File ${file.metadata?.name} ${file.chunks_stored} / ${file.chunks} has been uploaded`
      )

      /// Get the index of uploading file if it exists
      const index = uploading.value.findIndex((f) => f.id === file.id)

      if (index === -1) {
        console.log(`File ${file.metadata?.name} not found in the uploading list, adding...`)

        // File hasn't been found in the uploading list so we add it
        uploading.value.push(file)

        if (storage && keypair && storage.dir) {
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

        if (storage && keypair && storage.dir) {
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
            upload(file, tracker).catch((err) => {
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

/**
 * Upload single file from the upload queue
 */
export async function upload(file: UploadAppFile, progress?: ProgressFunction) {
  if (!file.started_upload_at) {
    file.started_upload_at = new Date()
  }
  if (progress) {
    await progress(file, false)
  }

  for (let chunk = 0; chunk < file.chunks; chunk++) {
    // Skip already uploaded chunks
    if (file.uploaded_chunks?.includes(chunk)) {
      if (progress) {
        await progress(file, chunk === file.chunks - 1)
      }

      continue
    }

    const data = await sliceChunk(file.file, chunk)

    file = await uploadChunk(file, data, chunk)

    console.log(`Uploaded chunk ${chunk} / ${file.chunks} of ${file.file.name}`)

    if (progress) {
      await progress(file, chunk === file.chunks - 1)
    }
  }

  return file
}

/**
 * Upload a single file chunk
 */
async function uploadChunk(
  file: UploadAppFile,
  data: Uint8Array,
  chunk: number,
  attempt: number = 0
): Promise<UploadAppFile> {
  if (!file.metadata?.key?.password) {
    throw new Error(`File ${file.id} is missing key`)
  }

  const encrypted = cryptfns.aes.encrypt(data, file.metadata?.key)
  const checksum = cryptfns.sha256.digest(encrypted)

  const query: Query = {
    chunk,
    checksum
  }

  const headers = {
    'Content-Type': 'application/octet-stream'
  }

  try {
    console.log(
      `Uploading chunk ${chunk} / ${file.chunks} of ${file.file.name} - upload attempt ${attempt}`
    )

    const uploaded = await Api.upload<AppFile>(`/api/storage/${file.id}`, encrypted, query, headers)

    return {
      ...uploaded,
      metadata: file.metadata,
      file: file.file,
      started_upload_at: file.started_upload_at || new Date()
    }
  } catch (err) {
    const error = err as ErrorResponse<Uint8Array>

    // If we get checksum error, most likely the data was corrupted during transfer
    // we wont retry indefinitely, but we will try a few times
    if (error.validation?.checksum && attempt < MAX_RETRIES) {
      console.warn(
        `Failed uploading chunk ${chunk} / ${file.chunks} of ${file.file.name}, failed checksum, retrying...`
      )
      return uploadChunk(file, data, chunk, attempt + 1)
    }

    // The chunk was already uploaded, so we can just return the file
    if (error.validation?.chunk === 'chunk_already_exists') {
      console.warn(
        `Failed uploading chunk ${chunk} / ${file.chunks} of ${file.file.name}, chunk already exist, skipping...`
      )
      return file
    }

    console.error(
      `Failed uploading chunk ${chunk} / ${file.chunks} of ${file.file.name}, either some unexpected error, or too many failed checksum tries, aborting...`,
      err
    )

    throw err
  }
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
