<script setup lang="ts">
import ActionsDropdown from '@/components/files/browser/ActionsDropdown.vue'
import FileThumbnail from '@/components/files/FileThumbnail.vue'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import TruncatedSpan from '@/components/ui/TruncatedSpan.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { formatPrettyDate, formatSize } from '!'
import {
  mdiDotsVertical,
  mdiCloudSyncOutline,
  mdiFolderAccount,
  mdiShareVariantOutline
} from '@mdi/js'
import type { AppFile } from 'types'
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { isPreviewable, isMarkdownFile } from '!/preview'
import { SHARED_WITH_ME_DIR_ID } from '!/storage'

const router = useRouter()

const isDropZone = ref(false)

const props = defineProps<{
  file: AppFile
  checkedRows: Partial<AppFile>[]
  hideDelete?: boolean
  share?: boolean
  hideCheckbox?: boolean
  highlighted?: boolean
  sizes: {
    checkbox: string
    name: string
    size: string
    type: string
    modifiedAt: string
    buttons: string
  }
}>()

const emits = defineEmits<{
  (event: 'actions', file: AppFile): void
  (event: 'deselect-all'): void
  (event: 'details', file: AppFile): void
  (event: 'download', file: AppFile): void
  (event: 'sharing', file: AppFile): void
  (event: 'remove', file: AppFile): void
  (event: 'rename', file: AppFile): void
  (event: 'fork', file: AppFile): void
  (event: 'leave', file: AppFile): void
  (event: 'select-one', value: boolean, file: AppFile): void
  (event: 'upload-many', files: FileList, dirId?: string): void
}>()

const selectOne = (value: boolean) => {
  emits('select-one', value, props.file)
}

const checked = computed({
  get: () => !!props.checkedRows.find((item) => item.id === props.file.id),
  set: (v) => selectOne(v)
})

const showProgress = computed(() => {
  if (props.file.mime === 'dir') {
    return false
  }

  return !props.file.finished_upload_at
})

const isDir = computed(() => {
  return props.file.mime === 'dir'
})

const fileName = computed(() => {
  const name = props.file.name || '...'

  return props.file.mime === 'dir' ? `${name}/` : name
})

const fileSize = computed(() => {
  return props.file.size ? formatSize(props.file.size) : ''
})

const fileModifiedAt = computed(() => {
  return props.file.file_modified_at ? formatPrettyDate(props.file.file_modified_at) : ''
})

const progressValue = computed(() => {
  const total = props.file.chunks

  if (!total || props.file.finished_upload_at) {
    return 100
  }

  const uploaded = props.file.chunks_stored || 0
  const progress = uploaded / total
  return Math.ceil(progress * 100)
})

const sharedClass = computed(() => {
  return 'dark:bg-brownish-900 hover:bg-dirty-white hover:dark:bg-brownish-700'
})

/**
 * Owner-of-row email surfaced next to the file name when the caller does
 * not own the row. `owner_email` is the server-side ground truth from the
 * storage listing; `shared_by_email` is the synthetic-root fallback the
 * incoming-share mapper sets when listing `__shared_with_me__`. Owned
 * rows skip the badge — the caller already knows whose folder they are
 * sitting in.
 */
const ownerBadgeEmail = computed(() => {
  if (props.file.is_owner) return ''
  return props.file.owner_email || props.file.shared_by_email || ''
})

/**
 * Inline hint next to the name when the caller (as owner) has shared
 * this row with at least one other account. Recipient-side rows surface
 * an "owned by" badge instead, so this stays hidden on incoming shares.
 */
const isSharedOut = computed(() => {
  if (!props.file.is_owner) return false
  return (props.file.shared_with_count ?? 0) > 0
})

const sharedOutTitle = computed(() => {
  const n = props.file.shared_with_count ?? 0
  if (n === 1) return 'Shared with 1 other account'
  return `Shared with ${n} other accounts`
})

/**
 * The synthetic "Shared with me" root is rendered as an injected
 * navigation affordance, not a real `user_files` row. Selecting or
 * actioning it would push the synthetic id into endpoints that demand a
 * UUID, so the checkbox and dropdown stay hidden — only the click-through
 * to navigate into the virtual folder remains.
 */
const isSyntheticRoot = computed(() => props.file.id === SHARED_WITH_ME_DIR_ID)

const border = 'sm:border-l sm:border-brownish-50 sm:dark:border-brownish-950'
const sizes = computed(() => {
  return {
    checkbox: `${props.sizes.checkbox}`,
    name: `${props.sizes.name}`,
    size: `${border} ${props.sizes.size}`,
    type: `${border} ${props.sizes.type}`,
    modifiedAt: `${border} ${props.sizes.modifiedAt}`,
    buttons: `${props.sizes.buttons} text-right`
  }
})

const clicks = ref(0)
const timer = ref()

/**
 * Click listener
 * that handles single and double clicks
 */
const click = () => {
  clicks.value++
  if (clicks.value === 1) {
    singleClick()

    timer.value = setTimeout(() => {
      clicks.value = 0
    }, 250)
  }

  if (clicks.value === 2) {
    clicks.value = 0
    clearTimeout(timer.value)
    doubleClick()
  }
}

const doubleClick = () => {
  if (isDir.value) {
    router.push({ name: 'files', params: { file_id: props.file.id } })
  } else {
    detailsOrPreview()
  }
}

const canWriteMarkdown = computed(() => {
  // Owners always; recipients only if their role allows writes. The
  // editor's save toolbar wires through `preview.editable` which is
  // the file row's editable flag — true for every markdown file — so
  // the routing decision is the place to gate Readers out of the
  // editor and into the read-only preview instead.
  if (props.file.is_owner) return true
  const role = props.file.share_role
  return role === 'editor' || role === 'co-owner'
})

const detailsOrPreview = () => {
  if (props.file.finished_upload_at && isMarkdownFile(props.file) && canWriteMarkdown.value) {
    router.push({ name: 'notes', params: { id: props.file.id } })
  } else if (props.file.finished_upload_at && isPreviewable(props.file)) {
    router.push({ name: 'file-preview', params: { id: props.file.id } })
  } else {
    emits('details', props.file)
  }
}

const singleClick = () => {
  if (isSyntheticRoot.value) return
  emits('deselect-all')
  selectOne(!checked.value)
}

const dragend = (e: DragEvent) => {
  if (props.file.mime !== 'dir') {
    return
  }

  isDropZone.value = false

  e.preventDefault()
  e.stopPropagation()
}

const dragover = (e: DragEvent) => {
  if (props.file.mime !== 'dir') {
    return
  }

  isDropZone.value = true

  e.preventDefault()
  e.stopPropagation()
}

const drop = (e: DragEvent) => {
  if (props.file.mime !== 'dir') {
    return
  }

  isDropZone.value = false

  e.preventDefault()
  e.stopPropagation()

  if (e.dataTransfer?.files && e.dataTransfer.files.length) {
    emits('upload-many', e.dataTransfer.files, props.file.id)

    setTimeout(() => {
      router.push({ name: 'files', params: { file_id: props.file.id } })
    }, 100)
  }
}
</script>

<template>
  <div
    @dragenter="dragover"
    @dragleave="dragend"
    @dragend="dragend"
    @dragover="dragover"
    @drop="drop"
    name="file-row"
    :data-testid="`file-row-${file.name}`"
    accesskey="test"
    class="w-full flex file-row-separator"
    :class="{
      'bg-brownish-50 dark:bg-brownish-700': !!checked,
      [sharedClass]: true,
      'outline-2 outline-redish-300 outline z-10': isDropZone
    }"
  >
    <div :class="sizes.checkbox">
      <TableCheckboxCell
        v-if="!props.hideCheckbox"
        v-model="checked"
        :disabled="isSyntheticRoot"
      />
    </div>

    <button
      :class="`${sizes.name} flex justify-start cursor-pointer prevent-select text-left`"
      :title="fileName"
      @click="click"
    >
      <FileThumbnail :file="file" img-class="w-6 h-6 mr-2 rounded-md" />

      <BaseIcon
        v-if="file.id === SHARED_WITH_ME_DIR_ID"
        :path="mdiFolderAccount"
        :size="18"
        class="mr-2 text-orangy-400"
        data-testid="shared-with-me-folder-icon"
      />

      <TruncatedSpan :text="fileName" />
      <span
        v-if="isSharedOut"
        class="ml-2 inline-flex items-center text-brownish-400 dark:text-brownish-300"
        :title="sharedOutTitle"
        data-testid="shared-out-badge"
      >
        <BaseIcon :path="mdiShareVariantOutline" :size="14" />
      </span>
      <!-- Saving-in-another-session badge. Surfaced as soon as the
           server reports a pending_version on the row so users get a
           heads-up before they try to edit and run into a 409. -->
      <span
        v-if="file.pending_version != null"
        class="ml-2 inline-flex items-center text-orangy-400"
        title="Another session is saving this note"
      >
        <BaseIcon :path="mdiCloudSyncOutline" :size="14" />
      </span>
      <span
        v-if="ownerBadgeEmail"
        class="ml-2 inline-flex items-center max-w-[10rem] truncate px-2 py-0.5 rounded-full text-[11px] uppercase tracking-wider bg-brownish-100 dark:bg-brownish-800 text-brownish-700 dark:text-brownish-200"
        :title="`Owned by ${ownerBadgeEmail}`"
        data-testid="shared-by-badge"
      >
        {{ ownerBadgeEmail }}
      </span>
    </button>

    <div :class="sizes.size" :title="fileSize">
      <span>{{ fileSize || '-' }}</span>
    </div>

    <div :class="sizes.type" :title="props.file.mime">
      <TruncatedSpan :text="props.file.mime" />
    </div>

    <div :class="sizes.modifiedAt" :title="fileModifiedAt">
      <TruncatedSpan :text="fileModifiedAt" />
    </div>

    <div :class="sizes.buttons">
      <template v-if="!isSyntheticRoot">
        <BaseButton
          class="ml-2 sm:hidden float-right"
          color="dark"
          :icon="mdiDotsVertical"
          small
          name="actions-modal"
          @click="emits('actions', file)"
          :disabled="!props.file.id"
        />
        <ActionsDropdown
          class="ml-2 hidden sm:block float-right"
          :model-value="props.file"
          :disabled="!props.file.id"
          :hide-delete="props.hideDelete"
          :share="props.share"
          @details="(f: AppFile) => emits('details', f)"
          @download="(f: AppFile) => emits('download', f)"
          @remove="(f: AppFile) => emits('remove', f)"
          @rename="(f: AppFile) => emits('rename', f)"
          @sharing="(f: AppFile) => emits('sharing', f)"
          @fork="(f: AppFile) => emits('fork', f)"
          @leave="(f: AppFile) => emits('leave', f)"
        />
      </template>
    </div>
  </div>

  <div
    v-if="showProgress"
    class="border-b-2 border-greeny-500 dark:border-greeny-400 -mt-[1px]"
    :style="{
      width: progressValue + '%'
    }"
  ></div>
</template>
<style lang="css">
.prevent-select {
  -webkit-user-select: none;
  -ms-user-select: none;
  user-select: none;
}

.file-row-separator {
  box-shadow: inset 0 -1px 0 0 rgba(120, 120, 120, 0.15);
}

.dark .file-row-separator {
  box-shadow: inset 0 -1px 0 0 rgba(120, 120, 120, 0.1);
}
</style>
