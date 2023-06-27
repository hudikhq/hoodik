<script setup lang="ts">
import ActionsDropdown from '@/components/files/browser/ActionsDropdown.vue'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import TruncatedSpan from '@/components/ui/TruncatedSpan.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { formatPrettyDate, formatSize } from '!'
import { mdiDotsVertical } from '@mdi/js'
import type { AppFile } from 'types'
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'

const router = useRouter()

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
  (event: 'details', file: AppFile): void
  (event: 'download', file: AppFile): void
  (event: 'link', file: AppFile): void
  (event: 'remove', file: AppFile): void
  (event: 'rename', file: AppFile): void
  (event: 'select-one', value: boolean, file: AppFile): void
  (event: 'deselect-all'): void
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

const border = 'sm:border-l-2 sm:border-brownish-50 sm:dark:border-brownish-950'
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

const detailsOrPreview = () => {
  if (props.file.finished_upload_at && props.file.thumbnail) {
    router.push({ name: 'file-preview', params: { id: props.file.id } })
  } else {
    emits('details', props.file)
  }
}

const singleClick = () => {
  emits('deselect-all')
  selectOne(!checked.value)
}
</script>

<template>
  <div
    class="w-full flex"
    :class="{
      'bg-brownish-50 dark:bg-brownish-700': !!checked,
      [sharedClass]: true
    }"
  >
    <div :class="sizes.checkbox">
      <TableCheckboxCell v-if="!props.hideCheckbox" v-model="checked" />
    </div>

    <button
      :class="`${sizes.name} flex justify-start cursor-pointer prevent-select text-left`"
      :title="fileName"
      @click="click"
    >
      <img
        name="thumbnail"
        v-if="file.thumbnail"
        :src="file.thumbnail"
        :alt="fileName"
        class="w-6 h-6 mr-2 rounded-md"
      />

      <TruncatedSpan :text="fileName" />
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
        @link="(f: AppFile) => emits('link', f)"
        @remove="(f: AppFile) => emits('remove', f)"
        @rename="(f: AppFile) => emits('rename', f)"
      />
    </div>
  </div>

  <div
    :class="{
      [sharedClass]: true,
      'border-b-2 border-greeny-800 dark:border-greeny-400 -pb-2': showProgress
    }"
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
</style>
