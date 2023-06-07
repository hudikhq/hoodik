<script setup lang="ts">
import { formatSize } from '!'
import FilePending from './FilePending.vue'
import FileDone from './FileDone.vue'
import FileFailed from './FileFailed.vue'
import FileRunning from './FileRunning.vue'
import { computed } from 'vue'
import type { DownloadAppFile, UploadAppFile, QueueItemActionType } from 'types'

const props = defineProps<{
  type: QueueItemActionType
  file: DownloadAppFile | UploadAppFile
}>()

const emits = defineEmits(['remove'])

const remove = () => {
  emits('remove', props.file, props.type)
}

const isUpload = computed(() => {
  return props.type.startsWith('upload')
})

const size = computed(() => {
  return formatSize(props.file.size || 0)
})

const error = computed(() => {
  if (!props.file.error) {
    return ''
  }

  if (props.file.error.context && typeof props.file.error.context === 'string') {
    return `${props.file.error.context}`
  }

  if (props.file.error.context) {
    return `Something went wrong`
  }

  return `${props.file.error}`
})

const name = computed<string>(() => {
  if (!props.file.name) {
    return `${props.file.id}`
  }

  return `${props.file.name || props.file.id}`
})

const titleText = computed(() => {
  if (error.value) {
    return error.value
  }

  return `${props.type.split(':')[0]}: ${name.value} (${size.value})`
})

const percentage = computed(() => {
  if (isUpload.value) {
    const stored = props.file.uploaded_chunks?.length || 0

    if (!stored) {
      return '0%'
    }

    return Math.round((stored / props.file.chunks) * 100) + '%'
  }

  const downloadedBytes = (props.file as DownloadAppFile).downloadedBytes || 0

  if (!props.file.size) {
    return '0%'
  }

  return Math.round((downloadedBytes / props.file.size) * 100) + '%'
})
</script>

<template>
  <div :title="titleText" class="flex justify-between items-center">
    <FilePending
      :file="props.file"
      :type="props.type"
      :name="name"
      :size="size"
      :is-upload="isUpload"
      v-if="props.type.endsWith('waiting')"
      @remove="remove"
    />

    <FileDone
      :file="props.file"
      :type="props.type"
      :name="name"
      :size="size"
      :is-upload="isUpload"
      v-if="props.type.endsWith('done')"
      @remove="remove"
    />

    <FileFailed
      :file="props.file"
      :type="props.type"
      :name="name"
      :size="size"
      :error="error"
      :is-upload="isUpload"
      v-if="props.type.endsWith('failed')"
      @remove="remove"
    />

    <FileRunning
      :file="props.file"
      :type="props.type"
      :name="name"
      :size="size"
      :is-upload="isUpload"
      v-if="props.type.endsWith('running')"
      @remove="remove"
    />
  </div>

  <div
    class="block border-t-4 border-greeny-800 dark:border-greeny-400"
    :style="{
      width: percentage
    }"
  ></div>
</template>
