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
import { failPendingDownloads, resolveDownloadBytes } from './storage/download/worker'

export const store = defineStore('queue', () => {
  const uploading = ref<IntervalType>()
  const downloading = ref<IntervalType>()
  const uploadWorkerListenerActive = ref(false)
  const downloadWorkerListenerActive = ref(false)

  /**
   * Start all the depending queues and setup worker listeners
   */
  async function start(files: FilesStore, upload: UploadStore, download: DownloadStore) {
    if ('Worker' in window) {
      // Dynamic on purpose: the spawn module references worker URLs that
      // only resolve under Vite, and environments without Worker (tests)
      // must never load it.
      const { ensureWorkers } = await import('./worker-spawn')
      ensureWorkers()
    } else {
      logger.warn('[queue] Worker API not available — transfers will run on main thread')
    }

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
            await handleDownloadCompletedMessage(download, event.data.response)
          }

          // Replies to the calls previews, forks and re-indexing make. Not
          // routed through the download store: these are reads with a return
          // value, not queued transfers.
          if (event.data.type === 'download-bytes') {
            resolveDownloadBytes(event.data.response)
          }
        }

        window.DOWNLOAD.onerror = (event) => {
          logger.error('[queue] DOWNLOAD worker error:', event)
          downloadWorkerListenerActive.value = false
          // A promise waiting on a dead worker never settles otherwise, and
          // the caller falls back to this thread on the next attempt.
          failPendingDownloads('The download worker stopped')
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
 *
 * The digest is keyed before anything touches the wire: the column stores it
 * under the file's search key — so any holder of the file key can still run
 * the resume equality check — and the same moment carries the digest tags
 * that make the file findable by pasting its digest into search. The bare
 * digest never leaves this function.
 */
export async function handleHashDoneMessage(files: FilesStore, id: string, sha256: string) {
  logger.info(`[queue] hash-done for ${id}: sha256=${sha256.slice(0, 8)}...`)

  // Keying failure degrades to "hashes not persisted", never to an error
  // that escapes this fire-and-forget handler into the page.
  let update: meta.KeyedHashesUpdate
  try {
    const { store: cryptoStore } = await import('./crypto')
    const keypair = cryptoStore().keypair

    // Navigating away can evict the row from the store before the digest of a
    // large file arrives, but the server row still wants its hash — fetch it
    // back rather than leaving the column empty forever.
    let current = files.getItem(id)
    if (!current?.key) {
      if (!keypair?.input) {
        logger.warn(`[queue] file ${id} has no key and no unlocked keypair — hashes not persisted`)
        return
      }
      current = await meta.get(keypair, id)
    }

    const cryptfns = await import('./cryptfns')
    const fileSearchKey = cryptfns.searchFileKey(current.key as Uint8Array)
    const keyed = cryptfns.searchTag(fileSearchKey, sha256)

    update = {
      sha256: keyed,
      search_tokens_file: [`${keyed}:1`]
    }

    // Only the owner can produce root-scope tags; an editor uploading into a
    // shared folder indexes the digest under the file scope alone.
    if (current.is_owner !== false && keypair?.input) {
      update.search_tokens_root = [
        `${cryptfns.searchTag(cryptfns.searchRootKey(keypair), sha256)}:1`
      ]
    }
  } catch (err) {
    logger.error('[queue] could not key the digest for', id, ':', err)
    return
  }

  try {
    await meta.updateHashes(id, update)
    logger.info(`[queue] updateHashes succeeded for ${id}`)
  } catch (err) {
    logger.error('[queue] Failed to persist hashes for', id, ':', err)
    return
  }

  // Re-read rather than spreading the earlier snapshot: the upload finishes
  // while the PUT above is in flight, and writing the pre-finish row back
  // would strip `finished_upload_at` from the UI — a file stuck looking
  // half-uploaded with most of its actions missing.
  const latest = files.getItem(id)
  if (latest) {
    files.updateItem({ ...latest, sha256: update.sha256 })
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
  const { transferableFile, chunkBytes, error, stage } = response

  await download.progress(files, transferableFile, chunkBytes, error, stage)
}

/**
 * Handle catching the file stream after it has completed with downloading
 * in the worker and send it to the browser download.
 */
async function handleDownloadCompletedMessage(
  download: DownloadStore,
  response: DownloadCompletedResponseMessage
) {
  const { transferableFile, blob } = response

  const url = window.URL.createObjectURL(blob)
  const anchor = document.createElement('a')
  anchor.href = url
  anchor.download = transferableFile.name
  anchor.click()
  window.URL.revokeObjectURL(url)

  // Only now has the browser actually received the file.
  download.finish(transferableFile)
}
