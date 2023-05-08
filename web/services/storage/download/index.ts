import { meta } from '..'
import * as sync from './sync'
import { FileMetadata } from '../metadata'
import { defineStore } from 'pinia'
import * as logger from '!/logger'

import type {
  DownloadAppFile,
  DownloadProgressFunction,
  FilesStore,
  IntervalType,
  ListAppFile,
  QueueStore
} from '../../../types'
import type { KeyPair } from 'types'
import { errorIntoWorkerError, localDateFromUtcString, utcStringFromLocal, uuidv4 } from '../..'
import { FILES_DOWNLOADING_AT_ONE_TIME, KEEP_FINISHED_DOWNLOADS_FOR_MINUTES } from '../constants'
import { ref } from 'vue'
import { startFileDownload } from '../workers'

export const store = defineStore('download', () => {
  /**
   * Start processing queue while its not stopped
   */
  async function start(storage: FilesStore, queue: QueueStore): Promise<IntervalType> {
    active.value = true

    logger.debug('Starting download queue')

    const tracker = (file: DownloadAppFile, chunkBytes: number) =>
      progress(storage, file, chunkBytes)

    return setInterval(async () => {
      if (active.value) {
        await _tick(tracker, queue)
      }
    }, 1000)
  }

  /**
   * Files ready for downloading
   */
  const waiting = ref<DownloadAppFile[]>([])

  /**
   * Files currently being downloaded
   */
  const running = ref<DownloadAppFile[]>([])

  /**
   * Files that failed the downloading process
   */
  const failed = ref<DownloadAppFile[]>([])

  /**
   * Files that finished the downloading process
   */
  const done = ref<DownloadAppFile[]>([])

  /**
   * Is the queue currently being processed
   */
  const active = ref(false)

  /**
   * Create function that will track the progress
   */
  async function progress(
    storage: FilesStore,
    file: DownloadAppFile,
    chunkBytes: number,
    error?: any
  ) {
    if (error) {
      file.error = error
      setFailed(file)
      return
    }

    if (done.value.filter((f) => f.temporaryId === file.temporaryId).length > 0) {
      return
    }

    if (failed.value.filter((f) => f.temporaryId === file.temporaryId).length > 0) {
      return
    }

    const currentFileId = file.file_id || null
    const currentDirId = storage?.dir?.id || null

    if (storage && currentFileId === currentDirId) {
      storage.upsertItem(file)
    }

    /// Get the index of downloading file if it exists
    const index = running.value.findIndex((f) => f.id === file.id)

    if (index === -1) {
      logger.debug(`File ${file.metadata?.name} not found in the downloading list, adding...`)

      // File hasn't been found in the downloading list so we add it
      running.value.push(file)
    }

    const item = running.value.splice(index, 1)[0]
    file.downloadedBytes = (item.downloadedBytes || 0) + chunkBytes

    // If the file has been finished, we will remove it from the downloading list
    // and move it to the done list
    if (file.downloadedBytes >= (file.size || 0)) {
      logger.debug(
        `File ${file.metadata?.name} has finished downloading, pushing to the done list...`
      )

      file.finished_downloading_at = utcStringFromLocal(new Date())
      done.value.push(file)

      return
    }

    // Update the file in the downloading list
    running.value.unshift(file)
  }

  /**
   * Run single tick of the upload queue that takes the waiting
   * files and starts the upload process for them
   */
  async function _tick(progress: DownloadProgressFunction, queue: QueueStore) {
    let batch: DownloadAppFile[] = []

    if (running.value.length < FILES_DOWNLOADING_AT_ONE_TIME) {
      batch = waiting.value.splice(0, FILES_DOWNLOADING_AT_ONE_TIME - running.value.length)
    }

    return new Promise((resolve) => {
      if (batch.length) {
        // We don't wait for this promise, it will be left to run in the background
        Promise.all(
          batch.map((file) => {
            download(file, queue, progress).catch((err) => {
              setFailed({ ...file, error: errorIntoWorkerError(err) })
            })
          })
        )
      }

      done.value = done.value.filter((file) => {
        if (file.finished_downloading_at) {
          const date =
            localDateFromUtcString(file.finished_downloading_at).valueOf() +
            KEEP_FINISHED_DOWNLOADS_FOR_MINUTES * 60 * 1000

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
  function setFailed(file: DownloadAppFile) {
    for (let i = 0; i < running.value.length; i++) {
      if (running.value[i].id === file.id) {
        running.value.splice(i, 1)

        break
      }
    }

    failed.value.push(file)
  }

  /**
   * Move the file back into the queue for another try at downloading
   */
  function retry(file: DownloadAppFile) {
    for (let i = 0; i < failed.value.length; i++) {
      if (failed.value[i].id === file.id) {
        failed.value.splice(i, 1)

        break
      }
    }

    waiting.value.push(file)
  }

  /**
   * Add new file to the download queue
   */
  async function push(file: ListAppFile) {
    return waiting.value.push({ ...file, temporaryId: uuidv4() })
  }

  /**
   * Cancel the upload of a file
   */
  async function cancel(files: FilesStore, file: DownloadAppFile) {
    if (running.value.filter((f) => f.id === file.id).length === 0) {
      throw new Error('File cannot be canceled when its not uploading')
    }

    file.cancel = true

    if ('DOWNLOAD' in window) {
      window.DOWNLOAD.postMessage({ type: 'cancel', kind: 'download', id: file.id })
    }
  }

  return {
    waiting,
    running,
    failed,
    done,
    active,
    cancel,
    start,
    retry,
    push,
    progress
  }
})

/**
 * Download the file and decrypt it chunked
 */
export async function download(
  file: DownloadAppFile,
  queue: QueueStore,
  progress?: DownloadProgressFunction
): Promise<void> {
  if (!file.metadata?.key) {
    throw new Error("File doesn't have a key, cannot decrypt the data, file is unrecoverable")
  }

  file.started_download_at = utcStringFromLocal()

  if (progress) {
    progress(file, 0)
  }

  // No way to download files in SW, so abandoned for now :(
  if (queue.downloadWorkerListenerActive) {
    await startFileDownload(file)
  } else {
    await sync.downloadAndDecryptStream(file, progress)
  }
}

/**
 * Get the file and the files content decrypt the file and its content
 */
export async function get(file: ListAppFile | string, kp: KeyPair): Promise<ListAppFile> {
  if (typeof file === 'string') {
    file = await meta.get(kp, file)
  }

  if (!file.metadata && file.encrypted_metadata && kp && kp.input) {
    file.metadata = await FileMetadata.decrypt(file.encrypted_metadata, kp)
  }

  if (!file.metadata?.key) {
    throw new Error("File doesn't have a key, cannot decrypt the data, file is unrecoverable")
  }

  file.data = await sync.downloadAndDecrypt(file)

  return file
}
