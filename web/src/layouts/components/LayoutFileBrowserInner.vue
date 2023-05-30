<script setup lang="ts">
import CreateDirectoryModal from '@/components/files/modals/CreateDirectoryModal.vue'
import DeleteMultipleModal from '@/components/files/modals/DeleteMultipleModal.vue'
import ActionsModal from '@/components/files/modals/ActionsModal.vue'
import DeleteModal from '@/components/files/modals/DeleteModal.vue'
import DetailsModal from '@/components/files/modals/DetailsModal.vue'
import UploadButton from '@/components/files/browser/UploadButton.vue'
import LinkModal from '@/components/modals/LinkModal.vue'
import { store as downloadStore } from '!/storage/download'
import { store as storageStore } from '!/storage'
import { store as cryptoStore } from '!/crypto'
import { store as linksStore } from '!/links'
import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import type { Authenticated, KeyPair, ListAppFile } from 'types'

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
  if (!route.params.file_id) {
    return null
  }

  return `${route.params.file_id}`
})

const Download = downloadStore()
const Storage = storageStore()
const Crypto = cryptoStore()
const Links = linksStore()

const openBrowseWindow = ref(false)
const isModalCreateDirActive = ref(false)
const isModalDeleteMultipleActive = ref(false)

const detailsView = ref<ListAppFile>()
const singleRemove = ref<ListAppFile>()
const actionFile = ref<ListAppFile>()
const linkView = ref<ListAppFile>()

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
 * Open a modal with file link, create it if it doesn't exist
 */
const link = (file: ListAppFile) => {
  if (file.mime === 'dir') {
    throw new Error('Cannot create link for a directory')
  }

  actionFile.value = undefined
  linkView.value = file
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
}

watch(
  () => parentId.value,
  () => load(),
  { immediate: true }
)
</script>
<template>
  <UploadButton v-model="openBrowseWindow" :dir="Storage.dir" :kp="Crypto.keypair" />
  <CreateDirectoryModal
    v-model="isModalCreateDirActive"
    :Crypto="Crypto"
    :Storage="Storage"
    @cancel="isModalCreateDirActive = false"
  />
  <ActionsModal v-model="actionFile" @remove="remove" @download="download" @details="details" />
  <DeleteModal v-model="singleRemove" :Storage="Storage" :kp="Crypto.keypair" />
  <DetailsModal v-model="detailsView" :Storage="Storage" :kp="Crypto.keypair" />
  <LinkModal v-model="linkView" :Storage="Storage" :Links="Links" :kp="Crypto.keypair" />
  <DeleteMultipleModal
    v-model="isModalDeleteMultipleActive"
    :Storage="Storage"
    :kp="Crypto.keypair"
  />

  <slot
    :authenticated="props.authenticated"
    :keypair="props.keypair"
    :parentId="parentId"
    :Storage="Storage"
    :download="Download"
    :loading="Storage.loading"
    :on="{
      actions,
      browse,
      details,
      directory,
      download,
      link,
      remove,
      'remove-all': removeAll,
      'select-one': Storage.selectOne,
      'select-all': Storage.selectAll
    }"
  />
</template>
