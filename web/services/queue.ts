import { defineStore } from 'pinia'
import type {
  FilesStore,
  IntervalType,
  UploadStore,
  DownloadStore,
  UploadChunkResponseMessage,
  DownloadProgressResponseMessage,
  DownloadCompletedResponseMessage
} from '../types'
import { ref } from 'vue'
import * as logger from '!/logger'
import * as meta from './storage/meta'

export const store = defineStore('queue', () => {
  const uploading = ref<IntervalType>()
  const downloading = ref<IntervalType>()
  const uploadWorkerListenerActive = ref(false)
  const downloadWorkerListenerActive = ref(false)

  /**
   * Start all the depending queues and setup worker listeners
   */
  async function start(files: FilesStore, upload: UploadStore, download: DownloadStore) {
    if (uploadWorkerListenerActive.value === false) {
      if ('UPLOAD' in window) {
        logger.info('[queue] UPLOAD worker found, attaching listener')
        uploadWorkerListenerActive.value = true

        window.UPLOAD.onmessage = async (event) => {
          logger.debug('[queue] UPLOAD worker message:', event.data.type)
          if (event.data.type === 'upload-progress') {
            await uploadMessage(files, upload, event.data.response)
          }
        }

        window.UPLOAD.onerror = (event) => {
          logger.error('[queue] UPLOAD worker error:', event)
          uploadWorkerListenerActive.value = false
        }

        setTimeout(() => {
          window.UPLOAD.postMessage({ type: 'ping', name: 'UPLOAD' })
        }, 100)
      } else {
        logger.warn('[queue] UPLOAD worker NOT available — uploads will use sync fallback on main thread')
      }
    }

    if (downloadWorkerListenerActive.value === false) {
      if ('DOWNLOAD' in window) {
        logger.info('[queue] DOWNLOAD worker found, attaching listener')
        downloadWorkerListenerActive.value = true

        window.DOWNLOAD.onmessage = async (event) => {
          downloadWorkerListenerActive.value = true

          logger.debug('[queue] DOWNLOAD worker message:', event.data.type)
          if (event.data.type === 'download-progress') {
            await handleDownloadProgressMessage(files, download, event.data.response)
          }

          if (event.data.type === 'download-completed') {
            await handleDownloadCompletedMessage(event.data.response)
          }
        }

        window.DOWNLOAD.onerror = (event) => {
          logger.error('[queue] DOWNLOAD worker error:', event)
          downloadWorkerListenerActive.value = false
        }

        setTimeout(() => {
          window.DOWNLOAD.postMessage({ type: 'ping', name: 'DOWNLOAD' })
        }, 100)
      } else {
        logger.warn('[queue] DOWNLOAD worker NOT available — downloads will use sync fallback on main thread')
      }
    }

    if (window.HASH) {
      logger.info('[queue] HASH worker found, attaching listener')
      window.HASH.onmessage = async (event) => {
        logger.debug('[queue] HASH worker message:', event.data?.type)
        if (event.data.type === 'hash-done') {
          await handleHashDoneMessage(files, event.data.id, event.data.sha256)
        }
        if (event.data.type === 'hash-error') {
          logger.error('[queue] Hash worker error for file', event.data.id, ':', event.data.error)
        }
      }

      window.HASH.onerror = (event) => {
        logger.error('[queue] Hash worker uncaught error:', event)
      }
    } else {
      logger.warn('[queue] HASH worker NOT available — SHA-256 will not be computed')
    }

    if (!uploading.value) {
      uploading.value = await upload.start(files, store())
    }

    if (!downloading.value) {
      downloading.value = await download.start(files, store())
    }
  }

  /**
   * Stop all the depending queues and remove worker listeners
   */
  function stop() {
    if (uploading.value) {
      clearInterval(uploading.value)
    }

    if (downloading.value) {
      clearInterval(downloading.value)
    }

    uploadWorkerListenerActive.value = false
    downloadWorkerListenerActive.value = false
  }

  return {
    uploadWorkerListenerActive,
    downloadWorkerListenerActive,
    start,
    stop
  }
})

/**
 * Called when the HASH worker finishes computing SHA-256 for an uploaded file.
 * Persists the hash to the server and updates the local storage entry so the
 * details modal reflects it without a page reload.
 */
async function handleHashDoneMessage(files: FilesStore, id: string, sha256: string) {
  logger.info(`[queue] hash-done for ${id}: sha256=${sha256.slice(0, 8)}...`)
  try {
    await meta.updateHashes(id, sha256)
    logger.info(`[queue] updateHashes succeeded for ${id}`)
  } catch (err) {
    logger.error('[queue] Failed to persist hashes for', id, ':', err)
    return
  }

  const current = files.getItem(id)
  if (current) {
    logger.debug(`[queue] updating store item ${id} with sha256`)
    files.updateItem({ ...current, sha256 })
  } else {
    logger.warn(`[queue] file ${id} not found in store — store not updated (server was updated)`)
  }
}

/**
 * Handle Worker event for received upload message
 */
async function uploadMessage(
  files: FilesStore,
  upload: UploadStore,
  response: UploadChunkResponseMessage
) {
  const storedChunks = response.transferableFile.uploaded_chunks?.length || 0

  await upload.progress(
    files,
    response.transferableFile,
    response.isDone || storedChunks === response.transferableFile.chunks,
    response.error
  )
}

/**
 * Handle and parse the message received from the worker about download progress
 */
async function handleDownloadProgressMessage(
  files: FilesStore,
  download: DownloadStore,
  response: DownloadProgressResponseMessage
) {
  const { transferableFile, chunkBytes, error } = response

  await download.progress(files, transferableFile, chunkBytes, error)
}

/**
 * Handle catching the file stream after it has completed with downloading
 * in the worker and send it to the browser download.
 */
async function handleDownloadCompletedMessage(response: DownloadCompletedResponseMessage) {
  const { transferableFile, blob } = response

  const url = window.URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = transferableFile.name
  anchor.click()
  window.URL.revokeObjectURL(url)
}
