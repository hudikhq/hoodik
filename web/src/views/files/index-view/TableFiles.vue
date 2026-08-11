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

const checkedIds = computed(() => new Set(props.selected.map((file) => file.id)))

const checkedRows = computed(() => {
  return props.items.filter((item) => checkedIds.value.has(item.id))
})

const listEl = ref<HTMLElement | null>(null)

/**
 * Anchor for shift-click. A plain click sets it; a shift-click fills in
 * everything between it and the clicked row, the way every file manager
 * behaves. Selection still flows out through `select-one`, so the parent
 * stays the single owner of what is checked.
 */
const anchorId = ref<string | null>(null)

const selectOneAt = (value: boolean, file: AppFile) => {
  anchorId.value = file.id
  emits('select-one', value, file)
}

const selectRange = (file: AppFile) => {
  const rows = props.items
  const to = rows.findIndex((f) => f.id === file.id)
  if (to < 0) return

  const from = anchorId.value ? rows.findIndex((f) => f.id === anchorId.value) : -1
  if (from < 0) {
    selectOneAt(true, file)
    return
  }

  const [start, end] = from <= to ? [from, to] : [to, from]
  for (let i = start; i <= end; i++) {
    if (!checkedIds.value.has(rows[i].id)) emits('select-one', true, rows[i])
  }
}

/**
 * Arrow keys walk the row buttons so the browser is operable without a
 * pointer. Enter is the button's own activation and Space is handled on the
 * row, which leaves only the movement to do here.
 */
const onListKeydown = (event: KeyboardEvent) => {
  if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return

  const target = event.target as HTMLElement | null
  if (!target?.hasAttribute('data-row-nav')) return

  const buttons = [...(listEl.value?.querySelectorAll<HTMLElement>('[data-row-nav]') ?? [])]
  const index = buttons.indexOf(target)
  const next = buttons[index + (event.key === 'ArrowDown' ? 1 : -1)]
  if (index < 0 || !next) return

  event.preventDefault()
  next.focus()
}

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

const borderClass = 'sm:border-l sm:border-paper-300 sm:dark:border-brownish-950'

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
    class="w-full p-2 mb-2 flex rounded-t-md bg-paper-100 dark:bg-brownish-900 gap-4"
    v-if="showActions"
  >
    <span
      v-if="checkedRows.length"
      data-testid="files-selected-count"
      class="self-center text-sm text-brownish-700 dark:text-brownish-50"
    >{{ $t('files.browser.selectedCount', { count: checkedRows.length }) }}</span>

    <BaseButton
      :title="$t('common.delete')"
      :iconSize="20"
      :xs="true"
      :icon="mdiTrashCanOutline"
      color="danger"
      v-if="showDeleteAll"
      @click="() => emits('remove-all')"
    />

    <BaseButton
      :title="$t('files.browser.addToDownloadQueue')"
      :iconSize="20"
      :xs="true"
      :icon="mdiDownloadMultiple"
      color="light"
      v-if="showDownloadMany"
      @click="() => emits('download-many')"
    />

    <BaseButton
      :title="$t('common.move')"
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
      :title="$t('files.browser.renameTitle')"
      :iconSize="20"
      :xs="true"
      :icon="mdiPencil"
      color="light"
      v-if="singleSelected"
      @click="() => emits('rename', singleSelected as AppFile)"
    />

    <BaseButton
      :title="$t('files.actions.preview')"
      :iconSize="20"
      :xs="true"
      :icon="mdiEye"
      color="light"
      v-if="singleSelected && isPreviewable(singleSelected)"
      :to="isMarkdownFile(singleSelected) ? { name: 'notes', params: { id: singleSelected.id } } : { name: 'file-preview', params: { id: singleSelected.id } }"
    />

    <BaseButton
      :title="$t('files.actions.details')"
      :iconSize="20"
      :xs="true"
      :icon="mdiInformationOutline"
      color="light"
      v-if="singleSelected"
      @click="() => emits('details', singleSelected as AppFile)"
    />

    <BaseButton
      :title="$t('files.actions.sharing')"
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
      :title="$t('files.browser.createDirectory')"
      :iconSize="20"
      :xs="true"
      :icon="mdiFolderPlusOutline"
      color="light"
      @click="emits('directory')"
      v-if="!checkedRows.length && !isSharedWithMeRoot && !isSharedFolder"
    />

    <BaseButton
      name="create-file"
      :title="$t('files.browser.newFile')"
      :iconSize="20"
      :xs="true"
      :icon="mdiFileDocumentPlusOutline"
      color="light"
      @click="emits('file')"
      v-if="!checkedRows.length && !isSharedWithMeRoot"
    />

    <BaseButton
      name="browse"
      :title="$t('files.browser.uploadFiles')"
      :iconSize="20"
      :xs="true"
      :icon="mdiFilePlusOutline"
      color="light"
      @click="emits('browse')"
      v-if="!checkedRows.length && !isSharedWithMeRoot"
    />

    <BaseButton
      name="browse-folder"
      :title="$t('files.browser.uploadFolder')"
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
    class="bg-white dark:bg-brownish-900 rounded-lg border border-paper-300/40 dark:border-brownish-700/40"
    @dragenter="dragover"
    @dragleave="dragend"
    @dragend="dragend"
    @dragover="dragover"
    @drop="drop"
  >
    <div class="w-full flex rounded-t-lg bg-paper-100 dark:bg-brownish-950 border-b border-paper-300 dark:border-brownish-700/40">
      <div :class="sizes.checkbox">
        <TableCheckboxCell v-model="checked" v-if="!props.hideCheckbox" :label="$t('common.selectAll')" />
      </div>

      <div :class="`${sizes.name}`">
        <SortableName
          name="name"
          :label="$t('common.name')"
          :sort-options="sortOptions"
          @sort="(v: string) => emits('set-sort-simple', v)"
        />
      </div>

      <div :class="`${sizes.size} ${borderClass}`">
        <SortableName
          name="size"
          :label="$t('common.size')"
          :sort-options="sortOptions"
          @sort="(v: string) => emits('set-sort-simple', v)"
        />
      </div>

      <div :class="`${sizes.type} ${borderClass}`">
        <SortableName
          name="mime"
          :label="$t('common.type')"
          :sort-options="sortOptions"
          @sort="(v: string) => emits('set-sort-simple', v)"
        />
      </div>

      <div :class="`${sizes.modifiedAt} ${borderClass}`">
        <SortableName
          name="file_modified_at"
          :label="$t('common.modified')"
          :sort-options="sortOptions"
          @sort="(v: string) => emits('set-sort-simple', v)"
        />
      </div>

      <div :class="`${sizes.buttons}`"></div>
    </div>

    <div
      v-if="props.error"
      class="w-full rounded-b-lg bg-paper-50 dark:bg-brownish-900 py-10 flex flex-col items-center gap-3"
      data-testid="files-error"
    >
      <span class="text-sm text-brownish-300 dark:text-brownish-50 px-6 text-center">
        {{ $t(props.error) }}
      </span>
      <BaseButton color="info" small :label="$t('common.retry')" @click="emits('retry')" />
    </div>
    <!-- Cached rows for the target folder render immediately; this only covers
         a folder we know nothing about yet. Placeholder rows on the real
         columns say what is arriving and stop the header jumping when it does,
         which a spinner in the middle of the panel does neither of. -->
    <div
      v-else-if="props.loading && !props.items.length"
      class="w-full rounded-b-lg bg-paper-50 dark:bg-brownish-900"
      data-testid="files-loading"
      role="status"
      :aria-label="$t('common.loading')"
    >
      <div
        v-for="(width, index) in ['w-2/5', 'w-3/5', 'w-1/3', 'w-1/2', 'w-2/5']"
        :key="index"
        class="w-full flex file-row-separator animate-pulse"
        aria-hidden="true"
      >
        <div :class="sizes.checkbox">
          <div class="w-5 h-5 rounded bg-paper-200 dark:bg-brownish-700" />
        </div>
        <div :class="sizes.name">
          <div class="w-6 h-6 mr-2 rounded-md shrink-0 bg-paper-200 dark:bg-brownish-700" />
          <div class="h-4 rounded bg-paper-200 dark:bg-brownish-700" :class="width" />
        </div>
        <div :class="sizes.size">
          <div class="h-3 w-12 rounded bg-paper-200/70 dark:bg-brownish-700/60" />
        </div>
        <div :class="sizes.type">
          <div class="h-3 w-12 rounded bg-paper-200/70 dark:bg-brownish-700/60" />
        </div>
        <div :class="sizes.modifiedAt">
          <div class="h-3 w-28 rounded bg-paper-200/70 dark:bg-brownish-700/60" />
        </div>
        <div :class="sizes.buttons" />
      </div>
    </div>
    <div
      v-else-if="!props.items.length"
      class="w-full rounded-b-lg bg-paper-50 dark:bg-brownish-900 py-14 flex flex-col items-center gap-1"
      data-testid="files-empty"
    >
      <span class="text-brownish-300 dark:text-brownish-50">{{
        $t('files.browser.emptyFolder')
      }}</span>
      <span class="text-xs text-brownish-200 dark:text-brownish-50">
        {{ $t('files.browser.emptyFolderHint') }}
      </span>
    </div>
    <div v-else ref="listEl" class="flex flex-col rounded-b-lg" @keydown="onListKeydown">
      <template v-for="file in props.items" :key="file.id">
        <TableFileRowWatcher
          :file="file"
          :sizes="sizes"
          :checkedIds="checkedIds"
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
          @select-one="selectOneAt"
          @select-range="selectRange"
          @upload-many="(f: FileList, d?: string) => emits('upload-many', f, d)"
        />
      </template>
    </div>
  </div>
</template>
