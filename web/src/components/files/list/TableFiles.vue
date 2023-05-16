<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import TableFileRowWatcher from '@/components/files/list/TableFileRowWatcher.vue'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import SpinnerIcon from '@/components/ui/SpinnerIcon.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { mdiTrashCanOutline, mdiFolderPlusOutline, mdiFilePlusOutline } from '@mdi/js'
import type { ListAppFile } from 'types'

const props = defineProps<{
  forDelete: ListAppFile[]
  items: ListAppFile[]
  parents: ListAppFile[]
  dir: ListAppFile | null
  file_id?: string
  searchedFileId?: string
  hideCheckbox?: boolean
  hideDelete?: boolean
  showActions?: boolean
  loading?: boolean
}>()

const emits = defineEmits<{
  (event: 'actions', file: ListAppFile): void
  (event: 'download', file: ListAppFile): void
  (event: 'details', file: ListAppFile): void
  (event: 'preview', file: ListAppFile): void
  (event: 'remove', file: ListAppFile): void
  (event: 'browse'): void
  (event: 'directory'): void
  (event: 'remove-all', files: ListAppFile[], fileId: string | null | undefined): void
  (event: 'select-one', select: boolean, file: ListAppFile): void
  (event: 'select-all', files: ListAppFile[], fileId: string | null | undefined): void
}>()

const checked = ref(false)

const dirId = computed<string | null>(() => {
  if (props.dir) {
    return props.dir.id
  }

  return null
})

const checkedRows = computed(() => {
  return props.items.filter((item) => {
    return props.forDelete.find((file) => file.id === item.id)
  })
})

const showDeleteAll = computed(() => {
  return (checked.value || checkedRows.value.length > 0) && !props.hideDelete
})

const items = computed(() => {
  const directories = props.items.filter((item) => {
    if (item.mime !== 'dir') {
      return false
    }

    if (props.dir) {
      return item.file_id === props.dir.id
    }

    return item.file_id === null
  })

  const files = props.items.filter((item) => {
    if (item.mime === 'dir') {
      return false
    }

    if (props.dir) {
      return item.file_id === props.dir.id
    }

    return item.file_id === null
  })

  return [...directories, ...files]
})

watch(
  () => checkedRows.value,
  (value) => {
    if (value.length === 0) {
      checked.value = false
    }
  }
)

watch(
  () => checked.value,
  (value) => {
    if (value) {
      emits(
        'select-all',
        items.value.filter((item) => item.file_id === dirId.value),
        dirId.value
      )
    } else {
      emits('select-all', [], dirId.value)
    }
  }
)

const borderClass = 'sm:border-l-2 sm:border-brownish-50 sm:dark:border-brownish-900'

const sizes = {
  checkbox: 'pl-2 pt-3 w-10',
  name: 'w-10/12 p-2 pt-3 sm:w-7/12 xl:w-6/12 flex',
  size: 'hidden p-2 pt-3 md:block md:w-2/12 xl:w-1/12',
  type: 'hidden p-2 pt-3 xl:block xl:w-1/12',
  createdAt: 'hidden p-2 pt-3 sm:block sm:w-4/12 lg:w-3/12 xl:w-2/12',
  uploadedAt: 'hidden p-2 pt-3 xl:block xl:w-1/12',
  buttons: 'w-2/12 p-2 sm:w-1/12'
}
</script>

<template>
  <div
    class="w-full p-2 mb-2 flex rounded-t-md bg-brownish-100 dark:bg-brownish-900 gap-4"
    v-if="showActions"
  >
    <BaseButton
      title="Delete selected files and folders"
      :iconSize="20"
      :xs="true"
      :icon="mdiTrashCanOutline"
      color="danger"
      v-if="showDeleteAll"
      @click="() => emits('remove-all', checkedRows, props.file_id)"
    />

    <BaseButton
      name="create-dir"
      title="Create directory"
      :iconSize="20"
      :xs="true"
      :icon="mdiFolderPlusOutline"
      color="light"
      v-if="!showDeleteAll"
      @click="emits('directory')"
    />

    <BaseButton
      name="browse"
      title="Upload files"
      :iconSize="20"
      :xs="true"
      :icon="mdiFilePlusOutline"
      color="light"
      v-if="!showDeleteAll"
      @click="emits('browse')"
    />
  </div>

  <div class="w-full flex rounded-t-lg bg-brownish-100 dark:bg-brownish-950">
    <div :class="sizes.checkbox">
      <TableCheckboxCell v-model="checked" v-if="!props.hideCheckbox" />
    </div>

    <div :class="`${sizes.name}`">
      <span>Name</span>
    </div>

    <div :class="`${sizes.size} ${borderClass}`">
      <span>Size</span>
    </div>

    <div :class="`${sizes.type} ${borderClass}`">
      <span>Type</span>
    </div>

    <div :class="`${sizes.createdAt} ${borderClass}`">
      <span>Created</span>
    </div>

    <div :class="`${sizes.uploadedAt} ${borderClass}`">
      <span>Uploaded</span>
    </div>

    <div :class="`${sizes.buttons}`"></div>
  </div>

  <div
    v-if="props.loading"
    class="w-full pt-20 rounded-b-lg bg-brownish-100 dark:bg-brownish-900 h-52 text-center"
  >
    <span class="w-1/2 h-1/2">
      <SpinnerIcon :size="200" />
    </span>
  </div>
  <div v-else class="flex flex-col rounded-b-lg">
    <template v-for="file in items" :key="file.id">
      <TableFileRowWatcher
        :file="file"
        :sizes="sizes"
        :checkedRows="checkedRows"
        :hideCheckbox="props.hideCheckbox"
        :hideDelete="props.hideDelete"
        :highlighted="props.searchedFileId === file.id"
        @actions="(f: ListAppFile) => emits('actions', f)"
        @remove="(f: ListAppFile) => emits('remove', f)"
        @details="(f: ListAppFile) => emits('details', f)"
        @preview="(f: ListAppFile) => emits('preview', f)"
        @download="(f: ListAppFile) => emits('download', f)"
        @select-one="(v: boolean, f: ListAppFile) => emits('select-one', v, f)"
      />
    </template>
  </div>
</template>
