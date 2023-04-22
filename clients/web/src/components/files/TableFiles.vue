<script setup lang="ts">
import { ref, computed } from 'vue'
import TableFilesRow from '@/components/files/TableFilesRow.vue'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import CardBoxModal from '../ui/CardBoxModal.vue'
import { store as storageStore, type ListAppFile } from '@/stores/storage'
import { store as cryptoStore } from '@/stores/crypto'

const storage = storageStore()
const crypto = cryptoStore()

const props = defineProps<{
  items: ListAppFile[]
  parent: ListAppFile | null
  file_id?: number
}>()

const emits = defineEmits<{
  (event: 'download', file: ListAppFile): void
}>()

const download = (file: ListAppFile) => {
  emits('download', file)
}

const isModalRemoveActive = ref(false)
const fileToRemove = ref<Partial<ListAppFile> | null>(null)
const _checkedRows = ref<ListAppFile[]>([])

const parentId = computed<number | null>(() => {
  if (props.parent) {
    return props.parent.id
  }

  return null
})

const checkedRows = computed<ListAppFile[]>(() => {
  return _checkedRows.value?.filter((item) => item.file_id === parentId.value) || []
})

const items = computed(() => {
  const directories = props.items.filter((item) => {
    if (item.mime !== 'dir') {
      return false
    }

    if (props.parent) {
      return item.file_id === props.parent.id
    }

    return item.file_id === null
  })

  const files = props.items.filter((item) => {
    if (item.mime === 'dir') {
      return false
    }

    if (props.parent) {
      return item.file_id === props.parent.id
    }

    return item.file_id === null
  })

  return [...directories, ...files]
})

const selectOne = (value: boolean, file: ListAppFile) => {
  if (value && file) {
    _checkedRows.value.push(file)
  } else {
    _checkedRows.value = _checkedRows.value.filter((item) => item.id !== file.id)
  }
}

const selectAll = (value: boolean) => {
  console.log(value)
  _checkedRows.value = value ? items.value.filter((item) => item.file_id === parentId.value) : []
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

const removeFile = (file: Partial<ListAppFile>) => {
  fileToRemove.value = file
  isModalRemoveActive.value = true
}

const viewFile = async () => {}
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
        <th>
          <TableCheckboxCell
            type="td"
            class="py-0 px-0"
            :model-value="false"
            @update:model-value="selectAll"
          />
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
      <template v-for="file in items" :key="file.id">
        <TableFilesRow
          :file="file"
          :checkedRows="checkedRows"
          @remove="removeFile"
          @view="viewFile"
          @checked="selectOne"
          @download="download"
        />
      </template>
    </tbody>
  </table>
</template>
