<script setup lang="ts">
import { mdiDotsVertical } from '@mdi/js'
import BaseButton from '@/components/ui/BaseButton.vue'
import TableCheckboxCell from '@/components/ui/TableCheckboxCell.vue'
import TruncatedSpan from '@/components/ui/TruncatedSpan.vue'
import ActionsDropdown from '../browser/ActionsDropdown.vue'
import { formatPrettyDate, formatSize } from '!'
import type { ListAppFile } from 'types'
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'

const router = useRouter()

const props = defineProps<{
  file: ListAppFile
  checkedRows: Partial<ListAppFile>[]
  hideDelete?: boolean
  hideCheckbox?: boolean
  highlighted?: boolean
  sizes: {
    checkbox: string
    name: string
    size: string
    type: string
    createdAt: string
    uploadedAt: string
    buttons: string
  }
}>()

const emits = defineEmits<{
  (event: 'actions', file: ListAppFile): void
  (event: 'remove', file: ListAppFile): void
  (event: 'details', file: ListAppFile): void
  (event: 'preview', file: ListAppFile): void
  (event: 'download', file: ListAppFile): void
  (event: 'select-one', value: boolean, file: ListAppFile): void
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
  const name = props.file.metadata?.name || '...'

  return props.file.mime === 'dir' ? `${name}/` : name
})

const fileSize = computed(() => {
  return props.file.size ? formatSize(props.file.size) : ''
})

const fileCreatedAt = computed(() => {
  return props.file.file_created_at ? formatPrettyDate(props.file.file_created_at) : ''
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

const fileFinishedUploadAt = computed(() => {
  return props.file.finished_upload_at ? formatPrettyDate(props.file.finished_upload_at) : ''
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
    createdAt: `${border} ${props.sizes.createdAt}`,
    uploadedAt: `${border} ${props.sizes.uploadedAt}`,
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
    timer.value = setTimeout(() => {
      clicks.value = 0
      singleClick()
    }, 200)
  } else {
    clearTimeout(timer.value)
    clicks.value = 0
    doubleClick()
  }
}

const singleClick = () => {
  if (isDir.value) {
    router.push({ name: 'files', params: { file_id: props.file.id } })
  } else {
    detailsOrPreview()
  }
}

const detailsOrPreview = () => {
  if (props.file.finished_upload_at && props.file.metadata?.thumbnail) {
    emits('preview', props.file)
  } else {
    emits('details', props.file)
  }
}

const doubleClick = () => {
  checked.value = !checked.value
}
</script>

<template>
  <div
    class="w-full flex"
    :class="{
      'bg-greeny-100 dark:bg-greeny-900 hover:bg-greeny-200 hover:dark:bg-greeny-800':
        props.highlighted,
      [sharedClass]: true
    }"
  >
    <div :class="sizes.checkbox">
      <TableCheckboxCell v-if="!props.hideCheckbox" v-model="checked" />
    </div>

    <div :class="`${sizes.name} cursor-pointer prevent-select`" :title="fileName" @click="click">
      <img
        v-if="file.metadata?.thumbnail"
        :src="file.metadata?.thumbnail"
        :alt="fileName"
        class="w-6 h-6 mr-2 rounded-md"
      />

      <TruncatedSpan :middle="fileName.length > 50" :text="fileName" />
    </div>

    <div :class="sizes.size" :title="fileSize">
      <span>{{ fileSize || '-' }}</span>
    </div>

    <div :class="sizes.type" :title="props.file.mime">
      <TruncatedSpan :text="props.file.mime" />
    </div>

    <div :class="sizes.createdAt" :title="props.file.file_created_at">
      <TruncatedSpan :text="fileCreatedAt" />
    </div>

    <div :class="sizes.uploadedAt">
      <TruncatedSpan v-if="props.file.finished_upload_at" :text="fileFinishedUploadAt" />
      <progress
        class="self-center w-full"
        :max="100"
        :value="progressValue"
        v-else-if="props.file.mime !== 'dir'"
        title="Uploading... If you stop the upload, or it gets interrupted simply select the same file again and it will continue where it left off"
      />
    </div>

    <div :class="sizes.buttons">
      <BaseButton
        class="ml-2 sm:hidden float-right"
        color="dark"
        :icon="mdiDotsVertical"
        small
        @click="emits('actions', file)"
        :disabled="!props.file.id"
      />
      <ActionsDropdown
        class="ml-2 hidden sm:block float-right"
        :model-value="props.file"
        :disabled="!props.file.id"
        @remove="(f: ListAppFile) => emits('remove', f)"
        @details="(f: ListAppFile) => emits('details', f)"
        @preview="(f: ListAppFile) => emits('preview', f)"
        @download="(f: ListAppFile) => emits('download', f)"
      />
    </div>
  </div>

  <div
    :class="{
      [sharedClass]: true,
      'block sm:hidden': true,
      'border-b-2 border-greeny-800 dark:border-greeny-400': showProgress
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
