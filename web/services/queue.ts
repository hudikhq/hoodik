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
import { FileMetadata } from './storage/metadata'

export const store = defineStore('queue', () => {
  const uploading = ref<IntervalType>()
  const downloading = ref<IntervalType>()
  const uploadWorkerListenerActive = ref(false)
  const downloadWorkerListenerActive = ref(false)

  /**
   * Start all the depending queues and setup worker listeners
   */
  async function start(files: FilesStore, upload: UploadStore, download: DownloadStore) {
    if (!uploading.value) {
      uploading.value = await upload.start(files, store())
    }

    if (!downloading.value) {
      downloading.value = await download.start(files, store())
    }

    if (uploadWorkerListenerActive.value === false) {
      if ('UPLOAD' in window) {
        window.UPLOAD.postMessage({ type: 'ping' })

        window.UPLOAD.onmessage = async (event) => {
          if (event.data.type === 'pong') {
            uploadWorkerListenerActive.value = true
          }

          if (event.data.type === 'upload-progress') {
            await uploadMessage(files, upload, event.data.response)
          }
        }
      }
    }

    if (downloadWorkerListenerActive.value === false) {
      if ('DOWNLOAD' in window) {
        window.UPLOAD.postMessage({ type: 'ping' })

        window.DOWNLOAD.onmessage = async (event) => {
          if (event.data.type === 'pong') {
            downloadWorkerListenerActive.value = true
          }

          if (event.data.type === 'download-progress') {
            await handleDownloadProgressMessage(files, download, event.data.response)
          }

          if (event.data.type === 'download-completed') {
            await handleDownloadCompletedMessage(event.data.response)
          }
        }
      }
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
 * Handle Worker event for received upload message
 */
async function uploadMessage(
  files: FilesStore,
  upload: UploadStore,
  response: UploadChunkResponseMessage
) {
  response.transferableFile.metadata = FileMetadata.fromJson(response.metadataJson)

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
  const { transferableFile, metadataJson, chunkBytes, error } = response
  transferableFile.metadata = FileMetadata.fromJson(metadataJson)

  await download.progress(files, transferableFile, chunkBytes, error)
}

/**
 * Handle catching the file stream after it has completed with downloading
 * in the worker and send it to the browser download.
 */
async function handleDownloadCompletedMessage(response: DownloadCompletedResponseMessage) {
  const { transferableFile, metadataJson, blob } = response
  transferableFile.metadata = FileMetadata.fromJson(metadataJson)

  const url = window.URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = transferableFile.metadata?.name as string
  anchor.click()
  window.URL.revokeObjectURL(url)
}
