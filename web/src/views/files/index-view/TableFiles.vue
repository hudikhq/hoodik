<script setup lang="ts">
import {
  mdiTrashCanOutline,
  mdiFolderPlusOutline,
  mdiFilePlusOutline,
  mdiDownloadMultiple,
  mdiPencil,
  mdiEye,
  mdiInformationOutline
} from '@mdi/js'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import TableFileRowWatcher from './TableFileRowWatcher.vue'
import SpinnerIcon from '@/components/ui/SpinnerIcon.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import SortableName from '@/components/ui/SortableName.vue'
import { computed, ref, watch } from 'vue'
import type { AppFile } from 'types'

const props = defineProps<{
  selected: AppFile[]
  items: AppFile[]
  parents: AppFile[]
  dir: AppFile | null
  file_id?: string
  searchedFileId?: string
  hideCheckbox?: boolean
  hideDelete?: boolean
  share?: boolean
  showActions?: boolean
  loading?: boolean
  sortOptions: { parameter: string; order: string }
}>()

const emits = defineEmits<{
  (event: 'actions', file: AppFile): void
  (event: 'download', file: AppFile): void
  (event: 'link', file: AppFile): void
  (event: 'remove', file: AppFile): void
  (event: 'details', file: AppFile): void
  (event: 'rename', file: AppFile): void
  (event: 'browse'): void
  (event: 'directory'): void
  (event: 'select-one', select: boolean, file: AppFile): void
  (event: 'select-all', files: AppFile[], fileId: string | null | undefined): void
  (event: 'deselect-all'): void
  (event: 'download-many'): void
  (event: 'remove-all'): void
  (event: 'set-sort-simple', value: string): void
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
    return props.selected.find((file) => file.id === item.id)
  })
})

const showDeleteAll = computed(() => {
  return (checked.value || checkedRows.value.length > 0) && !props.hideDelete
})

const showDownloadMany = computed(() => {
  const hasDirsChecked = checkedRows.value.some((item) => item.mime === 'dir')
  const hasIncompleteUploads = checkedRows.value.some((item) => !item.finished_upload_at)

  return checkedRows.value.length > 0 && !hasDirsChecked && !hasIncompleteUploads
})

const singleSelected = computed(() => {
  if (checkedRows.value.length !== 1) {
    return null
  }

  return checkedRows.value[0]
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
      emits('select-all', props.items, dirId.value)
    } else {
      emits('select-all', [], dirId.value)
    }
  }
)

const borderClass = 'sm:border-l-2 sm:border-brownish-50 sm:dark:border-brownish-900'

const sizes = {
  checkbox: 'pl-2 pt-3 w-10',
  name: 'w-10/12 p-2 pt-3 sm:w-7/12 xl:w-7/12 flex',
  size: 'hidden p-2 pt-3 md:block md:w-2/12 xl:w-1/12',
  type: 'hidden p-2 pt-3 xl:block xl:w-1/12',
  modifiedAt: 'hidden p-2 pt-3 sm:block sm:w-4/12 lg:w-3/12 xl:w-2/12',
  buttons: 'w-2/12 p-2 sm:w-1/12'
}
</script>

<template>
  <div
    class="w-full p-2 mb-2 flex rounded-t-md bg-brownish-100 dark:bg-brownish-900 gap-4"
    v-if="showActions"
  >
    <BaseButton
      title="Delete"
      :iconSize="20"
      :xs="true"
      :icon="mdiTrashCanOutline"
      color="danger"
      v-if="showDeleteAll"
      @click="() => emits('remove-all')"
    />

    <BaseButton
      title="Add to download queue"
      :iconSize="20"
      :xs="true"
      :icon="mdiDownloadMultiple"
      color="light"
      v-if="showDownloadMany"
      @click="() => emits('download-many')"
    />

    <BaseButton
      title="Rename file or folder"
      :iconSize="20"
      :xs="true"
      :icon="mdiPencil"
      color="light"
      v-if="singleSelected"
      @click="() => emits('rename', singleSelected as AppFile)"
    />

    <BaseButton
      title="Preview"
      :iconSize="20"
      :xs="true"
      :icon="mdiEye"
      color="light"
      v-if="singleSelected && singleSelected.thumbnail"
      :to="{ name: 'file-preview', params: { id: singleSelected.id } }"
    />

    <BaseButton
      title="Details"
      :iconSize="20"
      :xs="true"
      :icon="mdiInformationOutline"
      color="light"
      v-if="singleSelected"
      @click="() => emits('details', singleSelected as AppFile)"
    />

    <BaseButton
      name="create-dir"
      title="Create directory"
      :iconSize="20"
      :xs="true"
      :icon="mdiFolderPlusOutline"
      color="light"
      @click="emits('directory')"
      v-if="!checkedRows.length"
    />

    <BaseButton
      name="browse"
      title="Upload files"
      :iconSize="20"
      :xs="true"
      :icon="mdiFilePlusOutline"
      color="light"
      @click="emits('browse')"
      v-if="!checkedRows.length"
    />
  </div>

  <div class="w-full flex rounded-t-lg bg-brownish-100 dark:bg-brownish-950">
    <div :class="sizes.checkbox">
      <TableCheckboxCell v-model="checked" v-if="!props.hideCheckbox" />
    </div>

    <div :class="`${sizes.name}`">
      <SortableName
        name="name"
        label="Name"
        :sort-options="sortOptions"
        @sort="(v: string) => emits('set-sort-simple', v)"
      />
    </div>

    <div :class="`${sizes.size} ${borderClass}`">
      <SortableName
        name="size"
        label="Size"
        :sort-options="sortOptions"
        @sort="(v: string) => emits('set-sort-simple', v)"
      />
    </div>

    <div :class="`${sizes.type} ${borderClass}`">
      <SortableName
        name="mime"
        label="Type"
        :sort-options="sortOptions"
        @sort="(v: string) => emits('set-sort-simple', v)"
      />
    </div>

    <div :class="`${sizes.modifiedAt} ${borderClass}`">
      <SortableName
        name="file_modified_at"
        label="Modified"
        :sort-options="sortOptions"
        @sort="(v: string) => emits('set-sort-simple', v)"
      />
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
    <template v-for="file in props.items" :key="file.id">
      <TableFileRowWatcher
        :file="file"
        :sizes="sizes"
        :checkedRows="checkedRows"
        :hideCheckbox="props.hideCheckbox"
        :hideDelete="props.hideDelete"
        :share="props.share"
        :highlighted="props.searchedFileId === file.id"
        @actions="(f: AppFile) => emits('actions', f)"
        @details="(f: AppFile) => emits('details', f)"
        @download="(f: AppFile) => emits('download', f)"
        @rename="(f: AppFile) => emits('rename', f)"
        @link="(f: AppFile) => emits('link', f)"
        @remove="(f: AppFile) => emits('remove', f)"
        @select-one="(v: boolean, f: AppFile) => emits('select-one', v, f)"
        @deselect-all="emits('deselect-all')"
      />
    </template>
  </div>
</template>
