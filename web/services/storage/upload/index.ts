import * as meta from '../meta'
import Api, { ErrorResponse } from '../../api'
import { errorIntoWorkerError, localDateFromUtcString, utcStringFromLocal, uuidv4 } from '../..'
import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as sync from './sync'
import { pushUploadToWorker } from '../workers'
import * as cryptfns from '../../cryptfns'
import { emitFileTreeChange } from '../events'
import {
  CHUNK_SIZE_BYTES,
  FILES_UPLOADING_AT_ONE_TIME,
  KEEP_FINISHED_UPLOADS_FOR_MINUTES
} from '../../constants'
import * as logger from '!/logger'

import type {
  CreateFile,
  UploadProgressFunction,
  UploadAppFile,
  IntervalType,
  FilesStore,
  KeyPair,
  QueueStore
} from 'types'
import { createThumbnail } from './thumbnail'

export const store = defineStore('upload', () => {
  /**
   * Start processing queue while its not stopped
   */
  async function start(storage: FilesStore, queue: QueueStore): Promise<IntervalType> {
    active.value = true

    logger.debug('Starting upload queue')

    const tracker = (file: UploadAppFile, isDone: boolean) => progress(storage, file, isDone)

    return setInterval(async () => {
      if (active.value) {
        await _tick(tracker, queue)
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
  const running = ref<UploadAppFile[]>([])

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
    const alreadyDone = done.value.some((f) => f.temporaryId === file.temporaryId)
    const alreadyFailed = failed.value.some((f) => f.temporaryId === file.temporaryId)

    // Hashes can arrive after the last chunk finishes (e.g. when using a separate hash worker).
    // In that case, the UI may have already moved the file to `done`, and we still want to
    // upsert the hash fields (md5/sha1/sha256/blake2b) without dropping `finished_upload_at`.
    if (alreadyDone || alreadyFailed) {
      const current = storage.getItem(file.id)
      if (current) {
        const merged = { ...current, ...file }

        // Preserve completion timestamps; later updates may not include them.
        if (alreadyDone) {
          merged.finished_upload_at = current.finished_upload_at
        }

        storage.updateItem(merged)
      }

      return
    }

    // Remove it from the uploading list
    running.value = running.value.filter((f) => f.id !== file.id)

    if (error) {
      file.error = error
      file.cancel = true
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
      logger.debug(`File ${file.name} is canceling the upload...`)

      running.value = running.value.filter((i) => i.id !== file.id)
      failed.value.push(file)
      return
    }

    // If the file has been finished, we will remove it from the uploading list
    // and move it to the done list
    if (isDone || file.finished_upload_at) {
      logger.info(`File "${file.name}" finished uploading`)

      file.finished_upload_at = Math.floor(new Date().valueOf() / 1000)
      done.value.push(file)
      emitFileTreeChange({ type: 'created', folderId: file.file_id || undefined })

      return
    }

    // Update the file in the uploading list
    running.value.unshift(file)
  }

  /**
   * Run single tick of the upload queue that takes the waiting
   * files and starts the upload process for them
   */
  async function _tick(tracker: UploadProgressFunction, queue: QueueStore) {
    let batch: UploadAppFile[] = []

    if (running.value.length < FILES_UPLOADING_AT_ONE_TIME) {
      batch = waiting.value.splice(0, FILES_UPLOADING_AT_ONE_TIME - running.value.length)
    }

    return new Promise((resolve) => {
      if (batch.length) {
        // We don't wait for this promise, it will be left to run in the background
        Promise.all(
          batch.map((file) => {
            logger.debug(
              'Pushing upload file to',
              queue.uploadWorkerListenerActive ? 'worker' : 'sync'
            )

            const promise = queue.uploadWorkerListenerActive
              ? pushUploadToWorker(file)
              : upload(file, tracker)

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
    for (let i = 0; i < running.value.length; i++) {
      if (running.value[i].id === file.id) {
        running.value.splice(i, 1)

        break
      }
    }

    failed.value.push(file)
  }

  /**
   * Add new file to the upload queue
   */
  async function push(keypair: KeyPair, file: File, parent_id?: string) {
    logger.info(`[upload:push] "${file.name}" (${(file.size / 1024 / 1024).toFixed(2)} MB)`)

    try {
      const existing = await meta.getByName(keypair, file.name, parent_id)

      const chunksStored = existing.chunks_stored || 0
      if (existing.chunks > chunksStored) {
        logger.info(
          `[upload:push] "${file.name}" resuming — ${chunksStored}/${existing.chunks} chunks done`
        )
        waiting.value.push({ ...existing, file, temporaryId: uuidv4() })
      } else {
        throw new Error('File already exists')
      }
    } catch (e) {
      if (!(e instanceof ErrorResponse) || e.status !== 404) {
        console.error('[upload:push] unexpected error before create:', e)
        throw e
      }

      let created: UploadAppFile
      try {
        created = await create(keypair, file, parent_id)
      } catch (createErr) {
        console.error('[upload:push] create failed:', createErr)
        throw createErr
      }

      logger.info(`[upload:push] "${file.name}" created as ${created.id}, queued for upload`)
      return waiting.value.push({ ...created, temporaryId: uuidv4() })
    }
  }

  /**
   * Cancel the upload of a file
   */
  async function cancel(files: FilesStore, file: UploadAppFile) {
    if (running.value.filter((f) => f.id === file.id).length === 0) {
      throw new Error('File cannot be canceled when its not uploading')
    }

    file.cancel = true

    if ('UPLOAD' in window) {
      window.UPLOAD.postMessage({ type: 'cancel', kind: 'upload', id: file.id })
    }
  }

  /**
   * Create new file metadata and add it to the upload queue
   */
  async function create(keypair: KeyPair, file: File, parent_id?: string): Promise<UploadAppFile> {
    const t0 = performance.now()
    const modified = file.lastModified ? new Date(file.lastModified) : new Date()

    logger.debug(`[upload:create] "${file.name}" — generating search tokens`)
    const search_tokens_hashed = cryptfns.stringToHashedTokens(file.name.toLowerCase())

    logger.debug(`[upload:create] "${file.name}" — generating thumbnail`)
    const thumbnail = await createThumbnail(file)

    logger.debug(
      `[upload:create] "${file.name}" — preparation done in ${(performance.now() - t0).toFixed(0)}ms`
    )

    const isMarkdown =
      file.name.toLowerCase().endsWith('.md') ||
      file.type === 'text/markdown' ||
      file.type === 'text/x-markdown'

    const createFile: CreateFile = {
      name: file.name,
      size: file.size,
      mime: file.type || (isMarkdown ? 'text/markdown' : 'application/octet-stream'),
      chunks: Math.ceil(file.size / CHUNK_SIZE_BYTES),
      file_id: parent_id,
      file_modified_at: utcStringFromLocal(modified),
      search_tokens_hashed,
      thumbnail,
      cipher: cryptfns.cipher.DEFAULT_CIPHER,
      editable: isMarkdown || undefined
    }

    const created = await meta.create(keypair, createFile)

    return { ...created, file }
  }

  return {
    waiting,
    running,
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
  logger.warn(`[upload:sync] "${file.name}" — using MAIN THREAD sync fallback (worker unavailable)`)

  if (!file.started_upload_at) {
    file.started_upload_at = utcStringFromLocal()
  }

  if (progress) {
    await progress(file, false)
  }

  const { token } = await meta.requestTransferToken(file.id, 'upload')
  const api = new Api({ ...new Api().toJson(), jwtToken: token, refreshToken: undefined })

  const workers = [...new Array(file.chunks)]
    .filter((_, c) => {
      return !file.uploaded_chunks?.includes(c)
    })
    .map((_, chunk) => {
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

        file = await sync.uploadChunk(file, data, chunk, 0, api)

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
