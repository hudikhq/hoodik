<script setup lang="ts">
import CreateDirectoryModal from '@/components/files/modals/CreateDirectoryModal.vue'
import CreateFileModal from '@/components/files/modals/CreateFileModal.vue'
import DeleteMultipleModal from '@/components/files/modals/DeleteMultipleModal.vue'
import MoveMultipleModal from '@/components/files/modals/MoveMultipleModal.vue'
import UploadButton from '@/components/files/browser/UploadButton.vue'
import DetailsModal from '@/components/files/modals/DetailsModal.vue'
import ActionsModal from '@/components/files/modals/ActionsModal.vue'
import RenameModal from '@/components/files/modals/RenameModal.vue'
import DeleteModal from '@/components/files/modals/DeleteModal.vue'
import SharingModal from '@/components/shares/SharingModal.vue'
import RevokeConfirmModal from '@/components/shares/RevokeConfirmModal.vue'
import type { Authenticated, KeyPair, AppFile } from 'types'
import { store as downloadStore } from '!/storage/download'
import { store as uploadStore } from '!/storage/upload'
import { store as storageStore } from '!/storage'
import { store as cryptoStore } from '!/crypto'
import { store as linksStore } from '!/links'
import {
  store as sharesStoreFactory,
  crypto as shareCrypto,
  fork as forkPipeline,
  ForkAbortedError
} from '!/shares'
import type { ForkProgress } from '!/shares'
import { SHARED_WITH_ME_DIR_ID } from '!/storage'
import { errorNotification, notification } from '!/index'
import * as meta from '!/storage/meta'
import { computed, ref, watch } from 'vue'
import { useTitle } from '@vueuse/core'
import { useRoute, useRouter } from 'vue-router'

const props = defineProps<{
  parentId?: string
  hideDelete?: boolean
  share?: boolean
  clear?: boolean
  authenticated: Authenticated
  keypair: KeyPair
}>()

const title = useTitle()
const route = useRoute()
const router = useRouter()
const parentId = computed(() => {
  if (props.parentId) {
    return props.parentId
  }

  if (!route.params.file_id) {
    return undefined
  }

  return `${route.params.file_id}`
})

const Upload = uploadStore()
const Download = downloadStore()
const Storage = storageStore()
const Crypto = cryptoStore()
const Links = linksStore()
const Shares = sharesStoreFactory()

const openBrowseWindow = ref(false)
const openFolderWindow = ref(false)
const isModalCreateDirActive = ref(false)
const isModalCreateFileActive = ref(false)
const isModalMoveMultipleActive = ref(false)
const isModalDeleteMultipleActive = ref(false)

const detailsFileId = ref<string | undefined>()
const sharingModalFileId = ref<string | null>(null)
const sharingModalInitialTab = ref<'people' | 'link'>('people')
// Use a computed so the modal sees the latest copy from storage (e.g.
// sha256 arriving after upload, or the encrypted-link blob landing after
// a link create). Plain ref captures stale references that diverge from
// the store after `replaceItem` splices in a new object.
const detailsFile = computed<AppFile | undefined>({
  get: () => (detailsFileId.value ? (Storage.getItem(detailsFileId.value) ?? undefined) : undefined),
  set: (value: AppFile | undefined) => {
    detailsFileId.value = value?.id
  }
})
const sharingModalFor = computed<AppFile | null>(() =>
  sharingModalFileId.value ? (Storage.getItem(sharingModalFileId.value) ?? null) : null
)
const singleRemoveFile = ref<AppFile>()
const actionFile = ref<AppFile>()
const renameFile = ref<AppFile>()
const leaveConfirmFile = ref<AppFile | null>(null)

const forkingId = ref<string | null>(null)
const forkProgress = ref<ForkProgress | null>(null)
let forkController: AbortController | null = null

/**
 * Opens a modal for a file to display actions
 * for it. This is used when the display is small and
 * the actions are hidden behind a ... button.
 */
const actions = (file: AppFile) => {
  actionFile.value = file
}

/**
 * Opens a modal to confirm removing a single file
 */
const details = (file: AppFile) => {
  actionFile.value = undefined
  detailsFileId.value = file.id
}


/**
 * Open a modal to rename a file
 */
const rename = (file: AppFile) => {
  actionFile.value = undefined
  renameFile.value = file
}

/**
 * Opens a modal to select a directory to move the selected files to
 */
const moveAll = () => {
  isModalMoveMultipleActive.value = true
}

/**
 * Opens a modal to confirm removing multiple files
 */
const removeAll = () => {
  isModalDeleteMultipleActive.value = true
}

/**
 * Opens a modal to confirm removing a single file
 */
const remove = (file: AppFile) => {
  actionFile.value = undefined
  singleRemoveFile.value = file
}

/**
 * Per-row Share opens the unified Sharing modal on the People tab.
 * Every Share affordance — row dropdown, DetailsModal title-bar button,
 * single-selection bulk toolbar — routes here, so the recipients list
 * always renders inline.
 */
const openShare = (file: AppFile) => {
  actionFile.value = undefined
  sharingModalFileId.value = file.id
  sharingModalInitialTab.value = 'people'
}

/**
 * Sends the file to the download queue
 */
const download = (file: AppFile) => {
  actionFile.value = undefined
  return Download.push(file)
}

/**
 * Co-owner-only fork action. Downloads + re-encrypts the shared
 * file under the caller's key chain so revocation by the original owner
 * no longer takes the copy away. The pipeline lives in
 * `services/shares/fork.ts` — this handler is the UI wrapper.
 */
const fork = async (file: AppFile) => {
  actionFile.value = undefined
  const user = props.authenticated.user
  if (!Crypto.keypair?.input || !Crypto.keypair?.publicKey) {
    errorNotification('Cannot save without an active session.')
    return
  }
  try {
    forkController = new AbortController()
    forkingId.value = file.id
    forkProgress.value = { bytesProcessed: 0, totalBytes: 0, phase: 'preparing' }
    const source = await meta.get(Crypto.keypair, file.id)
    const result = await forkPipeline.forkFile(
      {
        source,
        keypair: Crypto.keypair,
        callerUserId: user.id,
        callerRecipient: {
          pubkey: user.pubkey,
          key_type: user.key_type,
          wrapping_pubkey: user.wrapping_pubkey
        }
      },
      {
        signal: forkController.signal,
        onProgress: (progress) => {
          forkProgress.value = progress
        }
      }
    )
    notification(
      'Saved to your drive',
      `"${source.name}" is now your own file.`,
      'success'
    )
    try {
      const forked = await meta.get(Crypto.keypair, result.file_id)
      Storage.upsertItem(forked)
    } catch {
      // Pre-population is best-effort; the next navigation reloads.
    }
    router.push({ name: 'files' }).catch(() => {})
  } catch (err) {
    if (err instanceof ForkAbortedError) {
      notification('Save cancelled', 'The fork was cancelled before it completed.', 'info')
    } else {
      errorNotification(err)
    }
  } finally {
    forkingId.value = null
    forkProgress.value = null
    forkController = null
  }
}

const cancelFork = () => {
  if (forkController) forkController.abort()
}

/**
 * Recipient-side "remove from my drive" — drops only the caller's
 * user_files row via the same revoke endpoint with `recipient_id ===
 * caller`. Owner-side Delete stays on the regular `remove` modal.
 */
const openLeaveConfirm = (file: AppFile) => {
  actionFile.value = undefined
  leaveConfirmFile.value = file
}

const cancelLeave = () => {
  leaveConfirmFile.value = null
}

const confirmLeave = async () => {
  const file = leaveConfirmFile.value
  leaveConfirmFile.value = null
  if (!file) return
  const userId = props.authenticated.user.id
  if (!Crypto.keypair?.input) {
    errorNotification('Cannot remove yourself without an active session.')
    return
  }
  const role = file.share_role ?? 'reader'
  try {
    const timestamp = Math.floor(Date.now() / 1000)
    const signature = await shareCrypto.signAuditEvent(
      shareCrypto.buildAuditEventSigInput({
        senderId: userId,
        recipientId: userId,
        fileId: file.id,
        action: 'revoke',
        shareRoleBefore: role,
        shareRoleAfter: null,
        timestamp: BigInt(timestamp)
      }),
      Crypto.keypair.input
    )
    await Shares.revoke(file.id, userId, { event_signature: signature, timestamp })
    Storage.removeItem(file.id)
    notification('Removed from share', 'You no longer see this file.', 'success')
  } catch (err) {
    errorNotification(err)
  }
}

/**
 * Sends multiple selected files to the download queue if they are files
 * that have finished uploading
 */
const downloadMany = async () => {
  for (const file of Storage.selected) {
    if (file.mime === 'dir' || !file.finished_upload_at) {
      continue
    }

    await Download.push(file)
  }
}
/**
 * Takes the FileList object and adds all the selected files into upload queue.
 *
 * Branches on the target directory's ownership: when the parent is a
 * shared folder where the caller is NOT the owner, the upload routes
 * through `pushIntoSharedFolder` (multi-key fan-out) so every folder
 * member receives an RSA-wrapped copy of the new file's
 * key. Owned folders take the existing single-key `push` path.
 */
const uploadMany = async (files?: FileList, dirId?: string) => {
  if (!files) return

  const callerUserId = props.authenticated.user.id

  if (files.length) {
    const parentFolder = dirId ? Storage.getItem(dirId) : null
    // Use multi-key for any folder that's been shared, whether the
    // caller owns it or not. Owner-side uploads to a private folder
    // (no `members_signed_at`) stay on the regular create path because
    // multi-key requires a signed member list to verify against.
    const useMultiKey =
      parentFolder !== null &&
      parentFolder.mime === 'dir' &&
      (parentFolder.is_owner === false || parentFolder.members_signed_at != null)

    for (let i = 0; i < files.length; i++) {
      try {
        await files[i].slice(0, 1).arrayBuffer()
      } catch (err) {
        errorNotification(`File ${files[i].name} is a directory`)
        continue
      }

      try {
        if (useMultiKey && parentFolder) {
          await Upload.pushIntoSharedFolder(
            props.keypair,
            files[i],
            parentFolder,
            callerUserId
          )
        } else {
          await Upload.push(props.keypair, files[i], dirId)
        }
      } catch (error) {
        errorNotification(error)
      }
    }
  }

  if (!Upload.active) {
    Upload.active = true
  }
}

/**
 * Opens the file browser to select files
 */
const browse = () => {
  openBrowseWindow.value = true
}

/**
 * Opens the folder picker
 */
const browseFolder = () => {
  openFolderWindow.value = true
}

/**
 * Core folder upload logic. Takes a flat list of { file, relativePath } items,
 * creates the directory hierarchy as needed, then enqueues each file.
 */
async function uploadByPaths(
  items: { file: File; relativePath: string }[],
  baseDir: string | undefined
) {
  const dirCache = new Map<string, string>()

  // Shallowest paths first so parent dirs exist before children
  items.sort((a, b) => a.relativePath.split('/').length - b.relativePath.split('/').length)

  for (const { file, relativePath } of items) {
    const parts = relativePath.split('/')
    let currentParent = baseDir

    for (let i = 0; i < parts.length - 1; i++) {
      const pathKey = parts.slice(0, i + 1).join('/')

      if (!dirCache.has(pathKey)) {
        try {
          const existing = await meta.getByName(props.keypair, parts[i], currentParent)
          if (existing.mime === 'dir') {
            dirCache.set(pathKey, existing.id)
          } else {
            throw new Error(`"${parts[i]}" exists but is not a directory`)
          }
        } catch (e: any) {
          if (e?.status === 404) {
            const dir = await Storage.createDir(props.keypair, parts[i], currentParent)
            Storage.upsertItem(dir)
            dirCache.set(pathKey, dir.id)
          } else {
            errorNotification(e)
            return
          }
        }
      }

      currentParent = dirCache.get(pathKey)
    }

    try {
      const parentFolder = currentParent ? Storage.getItem(currentParent) : null
      if (
        parentFolder !== null &&
        parentFolder.mime === 'dir' &&
        (parentFolder.is_owner === false || parentFolder.members_signed_at != null)
      ) {
        await Upload.pushIntoSharedFolder(
          props.keypair,
          file,
          parentFolder,
          props.authenticated.user.id
        )
      } else {
        await Upload.push(props.keypair, file, currentParent)
      }
    } catch (e) {
      errorNotification(e)
    }
  }

  if (!Upload.active) {
    Upload.active = true
  }
}

/**
 * Handles folder upload from a webkitdirectory file picker.
 * Each File in the FileList has webkitRelativePath set by the browser.
 */
const uploadFolder = async (files: FileList, dirId?: string) => {
  if (!files?.length) return
  const items = Array.from(files).map((f) => ({
    file: f,
    relativePath: f.webkitRelativePath || f.name
  }))
  await uploadByPaths(items, dirId ?? parentId.value)
}

/**
 * Recursively reads all files from a FileSystemDirectoryEntry.
 */
async function readDirEntries(
  dirEntry: FileSystemDirectoryEntry,
  prefix: string
): Promise<{ file: File; relativePath: string }[]> {
  const results: { file: File; relativePath: string }[] = []
  const reader = dirEntry.createReader()
  const readBatch = () =>
    new Promise<FileSystemEntry[]>((res, rej) => reader.readEntries(res, rej))

  let batch: FileSystemEntry[]
  do {
    batch = await readBatch()
    for (const child of batch) {
      const childPath = `${prefix}/${child.name}`
      if (child.isFile) {
        const f = await new Promise<File>((res, rej) =>
          (child as FileSystemFileEntry).file(res, rej)
        )
        results.push({ file: f, relativePath: childPath })
      } else if (child.isDirectory) {
        results.push(...(await readDirEntries(child as FileSystemDirectoryEntry, childPath)))
      }
    }
  } while (batch.length > 0)

  return results
}

/**
 * Handles folder upload from drag-and-drop (FileSystemEntry[]).
 * Entries must be extracted synchronously inside the drop handler.
 */
const uploadFolderEntries = async (entries: FileSystemEntry[], dirId?: string) => {
  const items: { file: File; relativePath: string }[] = []

  for (const entry of entries) {
    if (entry.isFile) {
      const f = await new Promise<File>((res, rej) =>
        (entry as FileSystemFileEntry).file(res, rej)
      )
      items.push({ file: f, relativePath: entry.name })
    } else if (entry.isDirectory) {
      items.push(...(await readDirEntries(entry as FileSystemDirectoryEntry, entry.name)))
    }
  }

  await uploadByPaths(items, dirId ?? parentId.value)
}

/**
 * Opens a modal to create a new directory
 */
const directory = () => {
  isModalCreateDirActive.value = true
}

/**
 * Opens a modal to create a new markdown file
 */
const file = () => {
  isModalCreateFileActive.value = true
}

const load = async () => {
  // Snapshot the target before any await: `parentId` is reactive and the
  // user can navigate away while the calls below are in flight. Re-reading
  // it afterwards would bind the post-load decisions to wherever they have
  // since navigated, not the folder this load was for — which would skip
  // marking shares seen if the (slower) fetch resolves after a quick exit.
  const target = parentId.value

  // We are not loading when we have hash in the url
  // because that means we already have some files and
  // we want to scroll down to them.
  await Storage.find(Crypto.keypair, target, !route.hash)

  if (Storage.dir) {
    title.value = `${Storage.dir.name} -- ${window.defaultDocumentTitle}`
  }

  // Load or re-load the stats for the user so it can be properly
  await Storage.loadStats()

  // The virtual folder is the canonical destination for "look at what's
  // new", so reset the unread counter when the user opens it.
  if (target === SHARED_WITH_ME_DIR_ID) {
    Shares.markSeenNow()
  }
}

watch(
  () => parentId.value,
  () => load(),
  { immediate: true }
)
</script>
<template>
  <UploadButton
    v-model="openBrowseWindow"
    v-model:openFolder="openFolderWindow"
    :dir="Storage.dir"
    :kp="Crypto.keypair"
    @upload-many="(f: FileList) => uploadMany(f, parentId)"
    @upload-folder="(f: FileList) => uploadFolder(f, parentId)"
  />
  <RenameModal v-if="renameFile" v-model="renameFile" :Storage="Storage" :Crypto="Crypto" />
  <CreateDirectoryModal
    v-model="isModalCreateDirActive"
    :Crypto="Crypto"
    :Storage="Storage"
    @cancel="isModalCreateDirActive = false"
  />
  <CreateFileModal
    v-if="isModalCreateFileActive"
    v-model="isModalCreateFileActive"
    :Crypto="Crypto"
    :Storage="Storage"
    :authenticated-user-id="props.authenticated.user.id"
    @cancel="isModalCreateFileActive = false"
  />
  <ActionsModal
    v-model="actionFile"
    @remove="remove"
    @download="download"
    @details="details"
    @sharing="openShare"
    @rename="rename"
    @fork="fork"
    @leave="openLeaveConfirm"
  />
  <DeleteModal v-model="singleRemoveFile" :Storage="Storage" :kp="Crypto.keypair" />
  <DetailsModal v-model="detailsFile" />
  <SharingModal
    v-if="sharingModalFor && props.authenticated"
    :file="sharingModalFor"
    :authenticated-user-id="props.authenticated.user.id"
    :keypair="Crypto.keypair"
    :storage="Storage"
    :links="Links"
    :initial-tab="sharingModalInitialTab"
    @close="() => { sharingModalFileId = null }"
  />
  <DeleteMultipleModal
    v-model="isModalDeleteMultipleActive"
    :Storage="Storage"
    :kp="Crypto.keypair"
  />
  <MoveMultipleModal
    v-model="isModalMoveMultipleActive"
    :Storage="Storage"
    :kp="Crypto.keypair"
    :authenticated="props.authenticated"
  />

  <RevokeConfirmModal
    v-if="leaveConfirmFile"
    :model-value="true"
    recipient-label=""
    :is-self-remove="true"
    @update:model-value="(v) => { if (!v) cancelLeave() }"
    @confirm="confirmLeave"
    @cancel="cancelLeave"
  />

  <div
    v-if="forkingId && forkProgress"
    class="fixed bottom-4 right-4 z-50 p-3 rounded-lg bg-brownish-100 dark:bg-brownish-800 shadow-lg text-sm"
    data-testid="fork-progress"
  >
    <div class="flex items-center gap-2">
      <span>
        Saving to your drive · {{ forkProgress.phase }} ·
        {{
          Math.round(
            (forkProgress.bytesProcessed / Math.max(forkProgress.totalBytes, 1)) * 100
          )
        }}%
      </span>
      <button
        type="button"
        class="underline text-redish-500"
        data-testid="fork-progress-cancel"
        @click.prevent="cancelFork"
      >Cancel</button>
    </div>
  </div>

  <slot
    :authenticated="props.authenticated"
    :keypair="props.keypair"
    :parentId="parentId"
    :Storage="Storage"
    :download="Download"
    :loading="Storage.loading"
    :on="{
      'deselect-all': Storage.deselectAll,
      'download-many': downloadMany,
      'move-all': moveAll,
      'remove-all': removeAll,
      'select-all': Storage.selectAll,
      'select-one': Storage.selectOne,
      'set-sort-simple': Storage.setSortSimple,
      'upload-many': uploadMany,
      'browse-folder': browseFolder,
      'upload-folder-entries': uploadFolderEntries,
      actions,
      browse,
      details,
      directory,
      download,
      file,
      remove,
      rename,
      sharing: openShare,
      fork,
      leave: openLeaveConfirm
    }"
  />
</template>
