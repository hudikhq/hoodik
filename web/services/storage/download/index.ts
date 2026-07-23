import * as sync from './sync'
import { defineStore } from 'pinia'
import * as logger from '!/logger'

import type {
  DownloadAppFile,
  DownloadProgressFunction,
  FilesStore,
  IntervalType,
  AppFile,
  QueueStore
} from '../../../types'
import { errorIntoWorkerError, localDateFromUtcString, utcStringFromLocal, uuidv4 } from '../..'
import { FILES_DOWNLOADING_AT_ONE_TIME, KEEP_FINISHED_DOWNLOADS_FOR_MINUTES } from '../../constants'
import { ref } from 'vue'
import { startFileDownload } from '../workers'


// The browser row only needs to track transfer state coarsely — byte-level
// smoothness lives in the queue UI. Syncing the reactive listing on every
// chunk event re-sorted the open folder many times a second mid-transfer.
const lastRowSync = new Map<string, number>()

function shouldSyncRow(id: string, terminal: boolean): boolean {
  if (terminal) {
    lastRowSync.delete(id)
    return true
  }

  const now = Date.now()
  if (now - (lastRowSync.get(id) || 0) < 500) {
    return false
  }

  lastRowSync.set(id, now)
  return true
}

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
    error?: any,
    stage?: 'downloading' | 'processing'
  ) {
    if (error) {
      file.error = error
      logger.error(`File "${file.name}" download failed:`, error)
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

    if (storage && currentFileId === currentDirId && shouldSyncRow(file.id, !!file.error)) {
      storage.upsertItem(file)
    }

    // Remove any existing entry so we can re-insert at the front without duplicates.
    running.value = running.value.filter((f) => f.id !== file.id)

    // `chunkBytes` coming from the worker is `bytes_downloaded` (cumulative received bytes),
    // not a delta. Treat it as an absolute value to avoid double-counting and premature "done".
    file.downloadedBytes = chunkBytes
    file.stage = stage

    // Staged (worker) transfers stay in the running list until `finish`
    // moves them out — bytes reaching the total only means the pipeline
    // is done receiving, not that the blob has reached the browser. The
    // stage-less path keeps the old size check as its only completion
    // signal.
    if (!stage && file.downloadedBytes >= (file.size || 0)) {
      logger.debug(`File "${file.name}" finished downloading`)

      file.finished_downloading_at = utcStringFromLocal(new Date())
      done.value.push(file)

      return
    }

    // Keep the active download at the front of the list.
    running.value.unshift(file)
  }

  /**
   * The blob has been handed to the browser — the moment the transfer is
   * genuinely complete for the user.
   */
  function finish(file: DownloadAppFile) {
    if (done.value.some((f) => f.temporaryId === file.temporaryId)) {
      return
    }

    running.value = running.value.filter((f) => f.id !== file.id)

    file.stage = undefined
    file.downloadedBytes = file.size
    file.finished_downloading_at = utcStringFromLocal(new Date())
    done.value.push(file)
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
  async function push(file: AppFile) {
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
    finish,
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
  if (!file.key) {
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

