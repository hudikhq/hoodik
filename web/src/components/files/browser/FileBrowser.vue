<script setup lang="ts">
import { store as downloadStore } from '!/storage/download'
import { store as storageStore } from '!/storage'
import { store as cryptoStore } from '!/crypto'
import PreviewModal from '@/components/files/browser/PreviewModal.vue'
import DeleteMultipleModal from '@/components/files/browser/DeleteMultipleModal.vue'
import ActionsModal from '@/components/files/browser/ActionsModal.vue'
import CreateDirectoryModal from '@/components/files/browser/CreateDirectoryModal.vue'
import DeleteModal from '@/components/files/browser/DeleteModal.vue'
import DetailsModal from '@/components/files/browser/DetailsModal.vue'
import UploadButton from '@/components/files/browser/UploadButton.vue'
import { ref, watch, onMounted } from 'vue'
import type { ListAppFile } from 'types'
import { Helper } from '!/storage/helper'

const props = defineProps<{
  parentId?: string
  hideDelete?: boolean
  share?: boolean
}>()

const Download = downloadStore()
const storage = storageStore()
const crypto = cryptoStore()

const helper = ref<Helper>(new Helper(crypto.keypair, storage))

const openBrowseWindow = ref(false)
const isModalCreateDirActive = ref(false)
const isModalDeleteMultipleActive = ref(false)
const detailsView = ref<ListAppFile>()
const singleRemove = ref<ListAppFile>()
const actionFile = ref<ListAppFile>()
const previewFile = ref<ListAppFile>()

/**
 * Opens a modal for a file to display actions
 * for it. This is used when the display is small and
 * the actions are hidden behind a ... button.
 */
const actions = (file: ListAppFile) => {
  actionFile.value = file
}

/**
 * Opens a modal to confirm removing a single file
 */
const details = (file: ListAppFile) => {
  actionFile.value = undefined
  detailsView.value = file
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
const remove = (file: ListAppFile) => {
  actionFile.value = undefined
  singleRemove.value = file
}

/**
 * Sends the file to the download queue
 */
const download = (file: ListAppFile) => {
  actionFile.value = undefined
  return Download.push(file)
}

/**
 * Opens a preview view for certain files
 */
const preview = (file: ListAppFile) => {
  actionFile.value = undefined
  previewFile.value = file
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
  let file_id = null

  if (props.parentId !== undefined) {
    file_id = props.parentId
  } else {
    file_id = null
  }

  await storage.find(crypto.keypair, file_id)
}

watch(
  () => props.parentId,
  () => load()
)

onMounted(() => {
  load()
})
</script>
<template>
  <UploadButton v-model="openBrowseWindow" :dir="storage.dir" :kp="crypto.keypair" />
  <CreateDirectoryModal v-model="isModalCreateDirActive" @cancel="isModalCreateDirActive = false" />
  <ActionsModal
    v-model="actionFile"
    @remove="remove"
    @download="download"
    @preview="preview"
    @details="details"
  />
  <DeleteModal v-model="singleRemove" :storage="storage" :kp="crypto.keypair" />
  <DetailsModal v-model="detailsView" :storage="storage" :kp="crypto.keypair" />
  <DeleteMultipleModal
    v-model="isModalDeleteMultipleActive"
    :storage="storage"
    :kp="crypto.keypair"
  />
  <PreviewModal
    v-model="previewFile"
    :storage="storage"
    :kp="crypto.keypair"
    @download="download"
    @remove="remove"
    @details="details"
  />

  <slot
    :helper="helper"
    :parentId="parentId"
    :storage="storage"
    :download="Download"
    :loading="storage.loading"
    :on="{
      remove,
      download,
      actions,
      preview,
      browse,
      directory,
      details,
      'remove-all': removeAll,
      'select-one': storage.selectOne,
      'select-all': storage.selectAll
    }"
  />
</template>
