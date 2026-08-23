import * as meta from '../meta'
import Api, { ErrorResponse } from '../../api'
import { errorIntoWorkerError, localDateFromUtcString, utcStringFromLocal, uuidv4 } from '../..'
import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as sync from './sync'
import { finalizeUpload } from './direct'
// Straight from the module that defines it, for the same reason `./direct`
// does: the `!/shares` barrel imports this store back.
import { capabilitiesStore } from '!/shares/capabilities'
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
  AppFile,
  CreateFile,
  UploadProgressFunction,
  UploadAppFile,
  IntervalType,
  FilesStore,
  KeyPair,
  QueueStore
} from 'types'
import { createThumbnail } from './thumbnail'
import {
  uploadIntoSharedFolder,
  type UploadIntoSharedFolderArgs,
  type UploadIntoSharedFolderOptions,
  type UploadIntoSharedFolderProgress
} from '../../shares/editable'


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

export const store = defineStore('upload', () => {
  /**
   * Files the user cancelled, by id.
   *
   * Cancelling does not stop what is already in flight: chunk requests that
   * had left settle afterwards, and the worker reports each one. Those
   * reports carry the worker's own copy of the file, which never learned it
   * was cancelled, so they read as ordinary progress and put the row the user
   * just deleted back in the listing — a ghost that survives until a reload.
   * An id lands here the moment the user cancels and every later report for
   * it is dropped.
   */
  const cancelled = new Set<string>()

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
    // Nothing a cancelled upload reports can change anything: its row is gone
    // from the server and from the listing, and `cancel` has already moved it
    // to the failed list.
    if (cancelled.has(file.id)) {
      running.value = running.value.filter((f) => f.id !== file.id)
      storage.removeItem(file.id)

      return
    }

    const alreadyDone = done.value.some((f) => f.temporaryId === file.temporaryId)
    const alreadyFailed = failed.value.some((f) => f.temporaryId === file.temporaryId)

    // A failure can arrive for a file already listed as done: the direct path
    // commits in a request of its own after the last chunk, and a late error
    // is the only word the user gets that the file is not on the server after
    // all. Take it out of `done` and let it fall through to the failure
    // handling below rather than merging it in as a finished row.
    if (alreadyDone && error) {
      done.value = done.value.filter((f) => f.temporaryId !== file.temporaryId)
      file.finished_upload_at = undefined
    }

    // Hashes can arrive after the last chunk finishes (e.g. when using a separate hash worker).
    // In that case, the UI may have already moved the file to `done`, and we still want to
    // upsert the hash fields (md5/sha1/sha256/blake2b) without dropping `finished_upload_at`.
    if ((alreadyDone && !error) || alreadyFailed) {
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

    // Stamp the completion timestamp BEFORE the upsert. The upload
    // worker reports `isDone` from the WASM transfer layer but does not
    // re-fetch the row, so the `file` it hands us lacks the server's
    // `finished_upload_at`. Without this the storage store would store
    // the row as still-pending and the UI would render a forever-
    // uploading state even after the last chunk landed.
    if (isDone && !file.finished_upload_at) {
      file.finished_upload_at = Math.floor(new Date().valueOf() / 1000)
    }

    const currentFileId = file.file_id || null
    const currentDirId = storage?.dir?.id || null

    // Upsert the item in the storage
    if (
      !file.cancel &&
      storage &&
      currentFileId === currentDirId &&
      shouldSyncRow(file.id, isDone || !!error)
    ) {
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

      // Until the worker acknowledges it, a dispatched file would belong to
      // no list at all, and so fall outside the concurrency limit.
      running.value.push(...batch)
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

    // A file can already be listed as done when it fails: the main-thread path
    // reports its last chunk before asking the server to commit, and the
    // commit is what can still fail. Listed in both places it reads as a
    // finished upload with an error next to it.
    done.value = done.value.filter((f) => f.id !== file.id)

    failed.value.push(file)
  }

  /**
   * Add a file to the queue when the parent directory is a shared folder
   * the caller does NOT own. Routes the metadata create through
   * `POST /api/storage/upload-multikey` so every current member of the
   * folder gets their own RSA-wrapped copy of the file key.
   *
   * The chunk-upload step that follows is identical to the regular
   * `push` flow — chunks land via `POST /api/storage/{file_id}` once the
   * metadata row exists.
   */
  async function pushIntoSharedFolder(
    keypair: KeyPair,
    file: File,
    parentFolder: AppFile,
    callerUserId: string,
    options: {
      onProgress?: (p: UploadIntoSharedFolderProgress) => void
      signal?: AbortSignal
      onUnknownMember?: UploadIntoSharedFolderArgs['onUnknownMember']
    } = {}
  ): Promise<UploadAppFile> {
    logger.info(
      `[upload:pushIntoSharedFolder] "${file.name}" (${(file.size / 1024 / 1024).toFixed(2)} MB)`
    )
    const created = await createInSharedFolder(
      keypair,
      file,
      parentFolder,
      callerUserId,
      options
    )
    waiting.value.push({ ...created, temporaryId: uuidv4() })
    return created
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
    cancelled.add(file.id)

    if ('UPLOAD' in window) {
      window.UPLOAD.postMessage({ type: 'cancel', kind: 'upload', id: file.id })
    }

    // Moved to failed here rather than waiting for the worker to report the
    // cancellation, because every report after this one is ignored.
    running.value = running.value.filter((f) => f.id !== file.id)
    if (!failed.value.some((f) => f.id === file.id)) {
      failed.value.push(file)
    }

    // Cancelling means the user does not want the file, so the part of it
    // already on the server goes too — including chunks an earlier attempt
    // left behind, since those are the same file and count against the same
    // quota. Left in place it is a row that lists as a partial upload for
    // ever, holds the name against a later attempt, and charges the user for
    // storage nothing will ever finish.
    //
    // A finished file is the one thing cancel must not touch: the row would
    // have to reach this call while still listed as running, and deleting a
    // complete upload is not what anyone means by cancel.
    if (file.finished_upload_at) {
      return
    }

    try {
      await meta.remove(file.id)
      files.removeItem(file.id)
    } catch (err) {
      // The upload is cancelled either way; a row that outlived its delete is
      // worth a line in the log, not an error in the user's face.
      logger.warn(`[upload:cancel] could not remove ${file.id}:`, err)
    }
  }

  /**
   * Create the file's metadata via the multi-key upload protocol — used
   * when the parent directory is a shared folder the caller does not own.
   * The resulting `UploadAppFile` is otherwise identical to what `create`
   * returns, so the rest of the queue/worker pipeline is oblivious to
   * which path produced it.
   *
   * The crypto pipeline lives in `services/shares/editable.ts`; this
   * helper handles the storage-layer adapter — building the placeholder
   * row the chunk uploader consumes.
   */
  async function createInSharedFolder(
    keypair: KeyPair,
    file: File,
    parentFolder: AppFile,
    callerUserId: string,
    options: {
      onProgress?: (p: UploadIntoSharedFolderProgress) => void
      signal?: AbortSignal
      onUnknownMember?: UploadIntoSharedFolderArgs['onUnknownMember']
    } = {}
  ): Promise<UploadAppFile> {
    if (!keypair.input || !keypair.publicKey) {
      throw new Error('Cannot upload without an active keypair')
    }
    const modified = file.lastModified ? new Date(file.lastModified) : new Date()
    const thumbnail = await createThumbnail(file)
    const isMarkdown =
      file.name.toLowerCase().endsWith('.md') ||
      file.type === 'text/markdown' ||
      file.type === 'text/x-markdown'
    const mime = file.type || (isMarkdown ? 'text/markdown' : 'application/octet-stream')
    const cipher = cryptfns.cipher.defaultCipher()
    const fileKey = await cryptfns.cipher.generateKey(cipher)
    const fileKeyHex = cryptfns.uint8.toHex(fileKey)
    const encryptedName = await cryptfns.cipher.encryptString(cipher, file.name, fileKey)
    const encryptedThumbnail = thumbnail
      ? await cryptfns.cipher.encryptString(cipher, thumbnail, fileKey)
      : undefined
    const nameHash = cryptfns.searchTag(cryptfns.searchRootKey(keypair), file.name)
    const chunks = Math.ceil(file.size / CHUNK_SIZE_BYTES)
    const newFileId = uuidv4()

    const { trustedFingerprintsStore } = await import('../../shares')
    const trusted = trustedFingerprintsStore()

    const uploadArgs: UploadIntoSharedFolderArgs = {
      callerUserId,
      callerPrivateKey: keypair.input,
      callerPublicKey: keypair.publicKey,
      payload: {
        newFileId,
        parentFileId: parentFolder.id,
        fileKeyHex,
        nameHash,
        encryptedName,
        encryptedThumbnail,
        mime,
        size: file.size,
        chunks,
        cipher,
        editable: isMarkdown || undefined,
        fileModifiedAt: utcStringFromLocal(modified),
        searchTokensRoot: cryptfns.searchTags(
          cryptfns.searchRootKey(keypair),
          file.name.toLowerCase()
        ),
        searchTokensFile: cryptfns.searchTags(
          cryptfns.searchFileKey(fileKey),
          file.name.toLowerCase()
        )
      },
      trustedFingerprints: trusted,
      onUnknownMember: options.onUnknownMember ?? (async () => true)
    }
    const uploadOptions: UploadIntoSharedFolderOptions = {
      signal: options.signal,
      onProgress: options.onProgress
    }
    const result = await uploadIntoSharedFolder(uploadArgs, uploadOptions)

    // Construct the same UploadAppFile shape the regular create path
    // emits so the queue + chunk-upload code is unchanged downstream.
    const placeholder: UploadAppFile = {
      id: result.file_id,
      user_id: callerUserId,
      is_owner: true,
      name_hash: nameHash,
      mime,
      size: file.size,
      chunks,
      file_id: parentFolder.id,
      file_modified_at: Math.floor(modified.getTime() / 1000),
      created_at: Math.floor(Date.now() / 1000),
      is_new: true,
      editable: isMarkdown,
      active_version: 1,
      encrypted_key: '',
      encrypted_name: encryptedName,
      encrypted_thumbnail: encryptedThumbnail,
      cipher,
      key: fileKey,
      name: file.name,
      thumbnail,
      file
    } as UploadAppFile
    return placeholder
  }

  /**
   * Create new file metadata and add it to the upload queue
   */
  async function create(keypair: KeyPair, file: File, parent_id?: string): Promise<UploadAppFile> {
    const t0 = performance.now()
    const modified = file.lastModified ? new Date(file.lastModified) : new Date()

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
      thumbnail,
      cipher: cryptfns.cipher.defaultCipher(),
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
    pushIntoSharedFolder,
    create,
    createInSharedFolder,
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

  // The chunk indexes still missing, kept as real indexes: filtering the
  // array and letting `map` hand out positions again would renumber chunk 3
  // of a resume as chunk 0 and write the wrong slice into the wrong slot.
  const missing = [...new Array(file.chunks).keys()].filter(
    (chunk) => !file.uploaded_chunks?.includes(chunk)
  )

  const workers = missing.map((chunk) => {
    return async () => {
      const data = await sliceChunk(file.file as File, chunk)

      file = await sync.uploadChunk(file, data, chunk, 0, api)

      // Never done from here, however many chunks have landed: on the direct
      // path the commit is still to come, and the commit is what can fail.
      // The one `progress(file, true)` this path makes is below, after it.
      if (progress) {
        await progress(file, false)
      }

      return file
    }
  })

  while (workers.length) {
    const batch = workers.splice(0, 1)
    file = await Promise.race(batch.map((worker) => worker()))
  }

  // Chunks that went straight into the bucket never told the server they
  // landed, so a resume with nothing left to PUT — or an upload that finished
  // through the relay after starting directly — still owes the commit. The
  // server treats a repeated finalize as a no-op, so on a deployment without
  // direct transfer this is skipped and everywhere else it is safe to say
  // once too often. Awaited like every other gate read: an unfetched store
  // fails closed and would silently skip the commit.
  const capabilities = capabilitiesStore()
  await capabilities.ensureFetched()
  if (capabilities.directTransfer) {
    const committed = await finalizeUpload(file, api)
    if (committed) {
      file = {
        ...file,
        ...committed,
        key: file.key,
        name: file.name,
        thumbnail: file.thumbnail,
        temporaryId: file.temporaryId,
        file: file.file
      }
    }
  }

  // Done only here — past the commit, or past the point where there was none
  // to make. A throw above never reaches this, and the caller's catch is what
  // marks the file failed.
  if (progress) {
    await progress(file, true)
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
