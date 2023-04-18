<script setup lang="ts">
import { ref } from 'vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import TableFilesRow from '@/components/ui/TableFilesRow.vue'
import type { AppFile } from '@/stores/storage/meta'
import { store as storageStore } from '@/stores/storage'
import { store as cryptoStore } from '@/stores/crypto'

const storage = storageStore()
const crypto = cryptoStore()

defineProps<{
  checkable?: Boolean
  file_id?: number
}>()

const isModalRemoveActive = ref(false)
const fileToRemove = ref<Partial<AppFile> | null>(null)

const checkedRows = ref<Partial<AppFile>[]>([])

const removeFile = (file: Partial<AppFile>) => {
  fileToRemove.value = file
  isModalRemoveActive.value = true
}

const confirmRemove = async () => {
  if (fileToRemove.value) {
    await storage.remove(crypto.keypair, fileToRemove.value)
    fileToRemove.value = null
  }
  isModalRemoveActive.value = false
}

const cancelRemove = async () => {
  fileToRemove.value = null
  isModalRemoveActive.value = false
}

const viewFile = async () => {}
</script>

<template>
  <CardBoxModal
    v-model="isModalRemoveActive"
    title="Please confirm"
    button="danger"
    buttonLabel="Delete"
    has-cancel
    @cancel="cancelRemove"
    @confirm="confirmRemove"
  >
    <p>
      You are about to delete <b>{{ fileToRemove?.metadata?.name }}</b>
    </p>
    <p>This is sample modal</p>
  </CardBoxModal>

  <div v-if="checkedRows.length" class="p-3 bg-gray-100/50 dark:bg-slate-800">
    <span
      v-for="checkedRow in checkedRows"
      :key="checkedRow.id"
      class="inline-block px-2 py-1 rounded-sm mr-2 text-sm bg-gray-100 dark:bg-slate-700"
    >
      {{ checkedRow.metadata?.name }}
    </span>
  </div>

  <table>
    <thead>
      <tr>
        <th v-if="checkable" />
        <th>Name</th>
        <th>Size</th>
        <th>Type</th>
        <th>Created</th>
        <th>Uploaded</th>
        <th />
      </tr>
    </thead>
    <tbody>
      <template v-for="file in storage.items" :key="file.id">
        <TableFilesRow :file="file" :checkable="checkable" @remove="removeFile" @view="viewFile" />
      </template>
    </tbody>
  </table>
</template>
