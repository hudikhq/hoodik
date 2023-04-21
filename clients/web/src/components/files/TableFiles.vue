<script setup lang="ts">
import { ref } from 'vue'
import TableFilesRow from '@/components/files/TableFilesRow.vue'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import CardBoxModal from '../ui/CardBoxModal.vue'
import { store as storageStore, type ListAppFile } from '@/stores/storage'
import { store as cryptoStore } from '@/stores/crypto'

const storage = storageStore()
const crypto = cryptoStore()

const props = defineProps<{
  items: ListAppFile[]
  checkable?: boolean
  file_id?: number
}>()

const emits = defineEmits<{
  (event: 'download', file: ListAppFile): void
}>()

const isModalRemoveActive = ref(false)
const fileToRemove = ref<Partial<ListAppFile> | null>(null)

const checkedRows = ref<Partial<ListAppFile>[]>([])

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

const removeFile = (file: Partial<ListAppFile>) => {
  fileToRemove.value = file
  isModalRemoveActive.value = true
}

const viewFile = async () => {}

const checkAll = async () => {
  if (checkedRows.value.length === storage.items.length) {
    checkedRows.value = []
  } else {
    checkedRows.value = storage.items
  }
}
</script>

<template>
  <CardBoxModal
    title="Deleting the file"
    button="danger"
    v-model="isModalRemoveActive"
    button-label="Yes, delete"
    :has-cancel="true"
    @cancel="cancelRemove"
    @confirm="confirmRemove"
  >
    <p>
      Are you sure you want to delete forever '{{ fileToRemove?.metadata?.name }}'
      <span v-if="fileToRemove?.mime === 'dir'"> directory</span>
      ?
    </p>
  </CardBoxModal>
  <table>
    <thead>
      <tr>
        <th v-if="checkable">
          <TableCheckboxCell type="td" v-if="checkable" @checked="checkAll" class="py-0 px-0" />
        </th>
        <th>Name</th>
        <th>Size</th>
        <th>Type</th>
        <th>Created</th>
        <th>Uploaded</th>
        <th />
      </tr>
    </thead>
    <tbody>
      <template v-for="file in props.items" :key="file.id">
        <TableFilesRow
          :file="file"
          :checkable="checkable"
          @remove="removeFile"
          @view="viewFile"
          @download="emits('download', file)"
        />
      </template>
    </tbody>
  </table>
</template>
