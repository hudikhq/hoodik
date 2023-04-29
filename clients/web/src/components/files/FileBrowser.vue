<script setup lang="ts">
import { store as downloadStore } from '@/stores/storage/download'
import { store as uploadStore } from '@/stores/storage/upload'
import { store as storageStore } from '@/stores/storage'
import { store as cryptoStore } from '@/stores/crypto'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import CreateDirectoryModal from '@/components/files/CreateDirectoryModal.vue'
import { ref, watch, onMounted } from 'vue'
import type { ListAppFile } from '@/types'
import { Helper } from '@/stores/storage/helper'

const props = defineProps<{
  parentId?: string
}>()

const download = downloadStore()
const storage = storageStore()
const upload = uploadStore()
const crypto = cryptoStore()

const helper = ref<Helper>(new Helper(crypto.keypair, storage))

const isModalMultipleActive = ref(false)
const isModalCreateDirectory = ref(false)
const isModalSingleActive = ref(false)
const fileInput = ref<HTMLInputElement>()
const singleRemove = ref<ListAppFile>()

const confirmRemoveAll = async () => {
  await storage.removeAll(crypto.keypair, storage.forDelete)
  isModalMultipleActive.value = false
}

const confirmRemove = async () => {
  storage.forDelete = []

  if (singleRemove.value) {
    await storage.remove(crypto.keypair, singleRemove.value)
  }

  isModalMultipleActive.value = false
}

const removeAll = () => {
  isModalMultipleActive.value = true
}

const remove = (file: ListAppFile) => {
  singleRemove.value = file
  isModalSingleActive.value = true
}

const downloadAction = (file: ListAppFile) => {
  return download.push(file)
}

const addFiles = async () => {
  if (fileInput.value && fileInput.value?.files?.length) {
    for (let i = 0; i < fileInput.value?.files?.length; i++) {
      try {
        await upload.push(crypto.keypair, fileInput.value?.files?.[i], storage.dir?.id || undefined)
      } catch (error) {
        // TODO: Add some kind of notifications store...
      }
    }
  }

  if (fileInput.value) {
    fileInput.value.value = ''
  }

  if (!upload.active) {
    upload.active = true
  }
}

const browseFiles = () => {
  if (fileInput.value) {
    fileInput.value.click()
  }
}

const createDirectory = () => {
  isModalCreateDirectory.value = true
}

const cancel = () => {
  isModalCreateDirectory.value = false
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
  <input style="display: none" type="file" ref="fileInput" multiple @change="addFiles" />
  <CreateDirectoryModal v-model="isModalCreateDirectory" @cancel="cancel" />

  <CardBoxModal
    title="Delete multiple files and directories"
    button="danger"
    v-model="isModalMultipleActive"
    button-label="Yes, delete"
    :has-cancel="true"
    @cancel="isModalMultipleActive = false"
    @confirm="confirmRemoveAll"
  >
    <template v-if="storage.forDelete && storage.forDelete.length > 1">
      <p>Are you sure you want to delete {{ storage.forDelete.length }} items?</p>
    </template>

    <template v-else v-for="file in storage.forDelete" :key="file.id">
      <p>
        Are you sure you want to delete forever '{{ file?.metadata?.name }}'
        <span v-if="file?.mime === 'dir'"> directory</span>
        ?
      </p>
    </template>
  </CardBoxModal>

  <CardBoxModal
    title="Delete file or directory"
    button="danger"
    v-model="isModalSingleActive"
    button-label="Yes, delete"
    :has-cancel="true"
    @cancel="isModalSingleActive = false"
    @confirm="confirmRemove"
  >
    <p v-if="singleRemove">
      Are you sure you want to delete forever '{{ singleRemove?.metadata?.name }}'
      <span v-if="singleRemove?.mime === 'dir'"> directory</span>
      ?
    </p>
  </CardBoxModal>
  <slot
    :helper="helper"
    :parentId="parentId"
    :storage="storage"
    :download="download"
    :loading="storage.loading"
    :on="{
      'remove-all': removeAll,
      remove: remove,
      download: downloadAction,
      'select-one': storage.selectOne,
      'select-all': storage.selectAll,
      'create-directory': createDirectory,
      'browse-files': browseFiles
    }"
  />
</template>
