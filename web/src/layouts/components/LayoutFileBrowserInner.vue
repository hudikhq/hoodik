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
import LinkModal from '@/components/links/modals/LinkModal.vue'
import type { Authenticated, KeyPair, AppFile } from 'types'
import { store as downloadStore } from '!/storage/download'
import { store as uploadStore } from '!/storage/upload'
import { store as storageStore } from '!/storage'
import { store as cryptoStore } from '!/crypto'
import { store as linksStore } from '!/links'
import { errorNotification } from '!/index'
import * as meta from '!/storage/meta'
import { computed, ref, watch } from 'vue'
import { useTitle } from '@vueuse/core'
import { useRoute } from 'vue-router'

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

const openBrowseWindow = ref(false)
const openFolderWindow = ref(false)
const isModalCreateDirActive = ref(false)
const isModalCreateFileActive = ref(false)
const isModalMoveMultipleActive = ref(false)
const isModalDeleteMultipleActive = ref(false)

const detailsFileId = ref<string | undefined>()
// Use a computed so the modal sees the latest copy from storage (e.g. sha256 arriving after upload).
const detailsFile = computed<AppFile | undefined>({
  get: () => (detailsFileId.value ? (Storage.getItem(detailsFileId.value) ?? undefined) : undefined),
  set: (value: AppFile | undefined) => {
    detailsFileId.value = value?.id
  }
})
const singleRemoveFile = ref<AppFile>()
const actionFile = ref<AppFile>()
const renameFile = ref<AppFile>()
const linkFile = ref<AppFile>()

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
 * Open a modal with file link, create it if it doesn't exist
 */
const link = (file: AppFile) => {
  if (file.mime === 'dir') {
    throw new Error('Cannot create link for a directory')
  }

  actionFile.value = undefined
  linkFile.value = file
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
 * Sends the file to the download queue
 */
const download = (file: AppFile) => {
  actionFile.value = undefined
  return Download.push(file)
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
 * Takes the FileList object and adds all the selected files into upload queue
 */
const uploadMany = async (files?: FileList, dirId?: string) => {
  if (!files) return

  if (files.length) {
    for (let i = 0; i < files.length; i++) {
      try {
        await files[i].slice(0, 1).arrayBuffer()
      } catch (err) {
        errorNotification(`File ${files[i].name} is a directory`)
        continue
      }

      try {
        await Upload.push(props.keypair, files[i], dirId)
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
      await Upload.push(props.keypair, file, currentParent)
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
  // We are not loading when we have hash in the url
  // because that means we already have some files and
  // we want to scroll down to them.
  await Storage.find(Crypto.keypair, parentId.value, !route.hash)

  if (Storage.dir) {
    title.value = `${Storage.dir.name} -- ${window.defaultDocumentTitle}`
  }

  // Load or re-load the stats for the user so it can be properly
  await Storage.loadStats()
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
    v-model="isModalCreateFileActive"
    :Crypto="Crypto"
    :Storage="Storage"
    @cancel="isModalCreateFileActive = false"
  />
  <ActionsModal v-model="actionFile" @remove="remove" @download="download" @details="details" />
  <DeleteModal v-model="singleRemoveFile" :Storage="Storage" :kp="Crypto.keypair" />
  <DetailsModal v-model="detailsFile" :Storage="Storage" :kp="Crypto.keypair" />
  <LinkModal v-model="linkFile" :Storage="Storage" :Links="Links" :kp="Crypto.keypair" />
  <DeleteMultipleModal
    v-model="isModalDeleteMultipleActive"
    :Storage="Storage"
    :kp="Crypto.keypair"
  />
  <MoveMultipleModal v-model="isModalMoveMultipleActive" :Storage="Storage" :kp="Crypto.keypair" />

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
      link,
      remove,
      rename
    }"
  />
</template>
