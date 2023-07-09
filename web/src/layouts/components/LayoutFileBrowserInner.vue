<script setup lang="ts">
import CreateDirectoryModal from '@/components/files/modals/CreateDirectoryModal.vue'
import DeleteMultipleModal from '@/components/files/modals/DeleteMultipleModal.vue'
import MoveMultipleModal from '@/components/files/modals/MoveMultipleModal.vue'
import ActionsModal from '@/components/files/modals/ActionsModal.vue'
import RenameModal from '@/components/files/modals/RenameModal.vue'
import DeleteModal from '@/components/files/modals/DeleteModal.vue'
import DetailsModal from '@/components/files/modals/DetailsModal.vue'
import UploadButton from '@/components/files/browser/UploadButton.vue'
import LinkModal from '@/components/modals/LinkModal.vue'
import { store as downloadStore } from '!/storage/download'
import { store as uploadStore } from '!/storage/upload'
import { store as storageStore } from '!/storage'
import { store as cryptoStore } from '!/crypto'
import { store as linksStore } from '!/links'
import { errorNotification } from '!/index'
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import type { Authenticated, KeyPair, AppFile } from 'types'

const props = defineProps<{
  parentId?: string
  hideDelete?: boolean
  share?: boolean
  clear?: boolean
  authenticated: Authenticated
  keypair: KeyPair
}>()

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
const isModalCreateDirActive = ref(false)
const isModalMoveMultipleActive = ref(false)
const isModalDeleteMultipleActive = ref(false)

const detailsFile = ref<AppFile>()
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
  detailsFile.value = file
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
 * Opens a modal to create a new directory
 */
const directory = () => {
  isModalCreateDirActive.value = true
}

const load = async () => {
  // We are not loading when we have hash in the url
  // because that means we already have some files and
  // we want to scroll down to them.
  await Storage.find(Crypto.keypair, parentId.value, !route.hash)

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
    :dir="Storage.dir"
    :kp="Crypto.keypair"
    @upload-many="(f: FileList) => uploadMany(f, parentId)"
  />
  <RenameModal v-if="renameFile" v-model="renameFile" :Storage="Storage" :Crypto="Crypto" />
  <CreateDirectoryModal
    v-model="isModalCreateDirActive"
    :Crypto="Crypto"
    :Storage="Storage"
    @cancel="isModalCreateDirActive = false"
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
      actions,
      browse,
      details,
      directory,
      download,
      link,
      remove,
      rename
    }"
  />
</template>
