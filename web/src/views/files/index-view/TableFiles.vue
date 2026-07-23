<script setup lang="ts">
import {
  mdiTrashCanOutline,
  mdiFolderPlusOutline,
  mdiFilePlusOutline,
  mdiFileDocumentPlusOutline,
  mdiFolderArrowUpOutline,
  mdiDownloadMultiple,
  mdiPencil,
  mdiEye,
  mdiInformationOutline,
  mdiFolderMove,
  mdiShareVariantOutline
} from '@mdi/js'
import { useCapability } from '@/composables/useCapability'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import SortableName from '@/components/ui/SortableName.vue'
import TableFileRowWatcher from './TableFileRowWatcher.vue'
import SpinnerIcon from '@/components/ui/SpinnerIcon.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { computed, ref, watch } from 'vue'
import type { AppFile } from 'types'
import { isPreviewable, isMarkdownFile } from '!/preview'
import { SHARED_WITH_ME_DIR_ID } from '!/storage'

const props = defineProps<{
  selected: AppFile[]
  items: AppFile[]
  parents: AppFile[]
  parentId: string | undefined
  dir: AppFile | null
  file_id?: string
  searchedFileId?: string
  hideCheckbox?: boolean
  hideDelete?: boolean
  share?: boolean
  showActions?: boolean
  loading?: boolean
  error?: string | null
  sortOptions: { parameter: string; order: string }
}>()

const emits = defineEmits<{
  (event: 'actions', file: AppFile): void
  (event: 'browse'): void
  (event: 'retry'): void
  (event: 'deselect-all'): void
  (event: 'details', file: AppFile): void
  (event: 'directory'): void
  (event: 'file'): void
  (event: 'download-many'): void
  (event: 'download', file: AppFile): void
  (event: 'move-all'): void
  (event: 'remove-all'): void
  (event: 'remove', file: AppFile): void
  (event: 'rename', file: AppFile): void
  (event: 'sharing', file: AppFile): void
  (event: 'fork', file: AppFile): void
  (event: 'leave', file: AppFile): void
  (event: 'select-all', files: AppFile[], fileId: string | null | undefined): void
  (event: 'select-one', select: boolean, file: AppFile): void
  (event: 'set-sort-simple', value: string): void
  (event: 'upload-many', files: FileList, dirId?: string): void
  (event: 'browse-folder'): void
  (event: 'upload-folder-entries', entries: FileSystemEntry[], dirId?: string): void
}>()

const checked = ref(false)
const isDropZone = ref(false)

const dirId = computed<string | undefined>(() => {
  if (props.dir) {
    return props.dir.id
  }

  return undefined
})

const checkedRows = computed(() => {
  return props.items.filter((item) => {
    return props.selected.find((file) => file.id === item.id)
  })
})

const showDeleteAll = computed(() => {
  return checkedRows.value.length > 0 && !props.hideDelete
})

const showMoveAll = computed(() => {
  return checkedRows.value.length > 0
})

const { sharingEnabled } = useCapability()

const showDownloadMany = computed(() => {
  const hasDirsChecked = checkedRows.value.some((item) => item.mime === 'dir')
  const hasIncompleteUploads = checkedRows.value.some((item) => !item.finished_upload_at)

  return checkedRows.value.length > 0 && !hasDirsChecked && !hasIncompleteUploads
})

/**
 * Inside the synthetic "Shared with me" folder there is no real parent
 * to upload into — the user must first navigate into one of the shared
 * folders surfaced as a row. Write actions stay hidden to keep that
 * affordance unambiguous.
 */
const isSharedWithMeRoot = computed(() => props.parentId === SHARED_WITH_ME_DIR_ID)

/**
 * Inside a shared folder (caller has a write share but doesn't own it),
 * file uploads go through the multi-key path. Creating a subdirectory
 * has no multi-key equivalent yet, so the directory affordance hides
 * until that endpoint exists — falling back to the regular create would
 * produce a `parent_directory_not_found` toast the user can't recover
 * from.
 */
const isSharedFolder = computed(() => {
  const d = props.dir
  if (!d) return false
  if (d.mime !== 'dir') return false
  return d.is_owner === false
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

const dragend = (e: DragEvent) => {
  isDropZone.value = false

  e.preventDefault()
  e.stopPropagation()
}

const dragover = (e: DragEvent) => {
  isDropZone.value = true

  e.preventDefault()
  e.stopPropagation()
}

const drop = (e: DragEvent) => {
  isDropZone.value = false

  e.preventDefault()
  e.stopPropagation()

  if (isSharedWithMeRoot.value) return

  // Extract FileSystemEntry objects synchronously — DataTransferItemList is only valid
  // during the event and becomes empty after the handler returns.
  if (e.dataTransfer?.items) {
    const entries: FileSystemEntry[] = []
    for (let i = 0; i < e.dataTransfer.items.length; i++) {
      const entry = e.dataTransfer.items[i].webkitGetAsEntry()
      if (entry) entries.push(entry)
    }
    if (entries.some((entry) => entry.isDirectory)) {
      emits('upload-folder-entries', entries, dirId.value)
      return
    }
  }

  if (e.dataTransfer?.files && e.dataTransfer.files.length) {
    emits('upload-many', e.dataTransfer.files, dirId.value)
  }
}

const borderClass = 'sm:border-l sm:border-brownish-50 sm:dark:border-brownish-950'

const sizes = {
  checkbox: 'pl-2 pt-3 w-10 shrink-0',
  name: 'flex-1 p-2 pt-3 min-w-0 flex',
  size: 'hidden p-2 pt-3 md:block w-24 shrink-0',
  type: 'hidden p-2 pt-3 xl:block w-24 shrink-0',
  modifiedAt: 'hidden p-2 pt-3 sm:block w-44 shrink-0',
  buttons: 'w-10 p-2 shrink-0'
}
</script>

<template>
  <div
    class="w-full p-2 mb-2 flex rounded-t-md bg-brownish-100 dark:bg-brownish-900 gap-4"
    v-if="showActions"
  >
    <span
      v-if="checkedRows.length"
      data-testid="files-selected-count"
      class="self-center text-sm text-brownish-700 dark:text-brownish-200"
    >{{ checkedRows.length }} selected</span>

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
      title="Move"
      data-testid="move-selected"
      :iconSize="20"
      :xs="true"
      :icon="mdiFolderMove"
      color="light"
      v-if="showMoveAll"
      @click="() => emits('move-all')"
    />

    <span class="p-1" v-if="showMoveAll && singleSelected">|</span>

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
      v-if="singleSelected && isPreviewable(singleSelected)"
      :to="isMarkdownFile(singleSelected) ? { name: 'notes', params: { id: singleSelected.id } } : { name: 'file-preview', params: { id: singleSelected.id } }"
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
      title="Sharing"
      :iconSize="20"
      :xs="true"
      :icon="mdiShareVariantOutline"
      color="light"
      v-if="
        singleSelected &&
        sharingEnabled &&
        singleSelected.id !== SHARED_WITH_ME_DIR_ID &&
        (
          singleSelected.is_owner === false ||
          singleSelected.mime === 'dir' ||
          !!singleSelected.finished_upload_at
        )
      "
      data-testid="bulk-sharing-button"
      @click="() => emits('sharing', singleSelected as AppFile)"
    />

    <BaseButton
      name="create-dir"
      title="Create directory"
      :iconSize="20"
      :xs="true"
      :icon="mdiFolderPlusOutline"
      color="light"
      @click="emits('directory')"
      v-if="!checkedRows.length && !isSharedWithMeRoot && !isSharedFolder"
    />

    <BaseButton
      name="create-file"
      title="New file"
      :iconSize="20"
      :xs="true"
      :icon="mdiFileDocumentPlusOutline"
      color="light"
      @click="emits('file')"
      v-if="!checkedRows.length && !isSharedWithMeRoot"
    />

    <BaseButton
      name="browse"
      title="Upload files"
      :iconSize="20"
      :xs="true"
      :icon="mdiFilePlusOutline"
      color="light"
      @click="emits('browse')"
      v-if="!checkedRows.length && !isSharedWithMeRoot"
    />

    <BaseButton
      name="browse-folder"
      title="Upload folder"
      :iconSize="20"
      :xs="true"
      :icon="mdiFolderArrowUpOutline"
      color="light"
      @click="emits('browse-folder')"
      v-if="!checkedRows.length && !isSharedWithMeRoot && !isSharedFolder"
    />
  </div>

  <div
    :class="{
      'border-2 border-redish-300 border-spacing-0 m-[-2px]': isDropZone
    }"
    class="bg-white dark:bg-brownish-900 rounded-lg border border-brownish-200/40 dark:border-brownish-700/40"
    @dragenter="dragover"
    @dragleave="dragend"
    @dragend="dragend"
    @dragover="dragover"
    @drop="drop"
  >
    <div class="w-full flex rounded-t-lg bg-brownish-100 dark:bg-brownish-950 border-b border-brownish-200 dark:border-brownish-700/40">
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
      v-if="props.error"
      class="w-full rounded-b-lg bg-brownish-50 dark:bg-brownish-900 py-10 flex flex-col items-center gap-3"
      data-testid="files-error"
    >
      <span class="text-sm text-brownish-300 dark:text-brownish-100 px-6 text-center">
        {{ props.error }}
      </span>
      <BaseButton color="info" small label="Retry" @click="emits('retry')" />
    </div>
    <!-- Cached rows for the target folder render immediately; the spinner
         only covers a folder we know nothing about yet. -->
    <div
      v-else-if="props.loading && !props.items.length"
      class="w-full pt-20 rounded-b-lg bg-brownish-50 dark:bg-brownish-900 h-52 text-center"
    >
      <span class="w-1/2 h-1/2">
        <SpinnerIcon :size="200" />
      </span>
    </div>
    <div
      v-else-if="!props.items.length"
      class="w-full rounded-b-lg bg-brownish-50 dark:bg-brownish-900 py-14 flex flex-col items-center gap-1"
      data-testid="files-empty"
    >
      <span class="text-brownish-300 dark:text-brownish-100">This folder is empty</span>
      <span class="text-xs text-brownish-200 dark:text-brownish-300">
        Drop files here or use the upload button to add some
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
          @deselect-all="emits('deselect-all')"
          @details="(f: AppFile) => emits('details', f)"
          @download="(f: AppFile) => emits('download', f)"
          @remove="(f: AppFile) => emits('remove', f)"
          @rename="(f: AppFile) => emits('rename', f)"
          @sharing="(f: AppFile) => emits('sharing', f)"
          @fork="(f: AppFile) => emits('fork', f)"
          @leave="(f: AppFile) => emits('leave', f)"
          @select-one="(v: boolean, f: AppFile) => emits('select-one', v, f)"
          @upload-many="(f: FileList, d?: string) => emits('upload-many', f, d)"
        />
      </template>
    </div>
  </div>
</template>
