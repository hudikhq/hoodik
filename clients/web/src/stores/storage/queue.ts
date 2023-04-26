import { defineStore } from 'pinia'
import type {
  FilesStore,
  IntervalType,
  UploadStore,
  DownloadStore,
  UploadChunkResponseMessage,
  DownloadProgressResponseMessage
} from '../types'
import { ref } from 'vue'
import { FileMetadata } from './metadata'

export const store = defineStore('queue', () => {
  const uploading = ref<IntervalType>()
  const downloading = ref<IntervalType>()
  const messageListenersActive = ref(false)

  /**
   * Start all the depending queues and setup worker listeners
   */
  async function start(files: FilesStore, upload: UploadStore, download: DownloadStore) {
    if (!uploading.value) {
      uploading.value = await upload.start(files)
    }

    if (!downloading.value) {
      downloading.value = await download.start(files)
    }

    if ('SW' in window && messageListenersActive.value === false) {
      window.SW.onmessage = async (event) => {
        if (event.data.type === 'upload-progress') {
          await uploadMessage(files, upload, event.data.response)
        }

        if (event.data.type === 'download-progress') {
          await handleDownloadProgressMessage(files, download, event.data.message)
        }
      }

      messageListenersActive.value = true
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
  }

  return {
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
  const { transferableFile, chunkBytes, error } = response

  await download.progress(files, transferableFile, chunkBytes, error)
}
