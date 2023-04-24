<script setup lang="ts">
import { formatSize, localDateFromUtcString } from '@/stores'
import type { DownloadAppFile, UploadAppFile } from '@/stores/storage/types'
import { computed } from 'vue'

const props = defineProps<{
  type:
    | 'upload:uploading'
    | 'upload:waiting'
    | 'upload:failed'
    | 'upload:done'
    | 'download:downloading'
    | 'download:waiting'
    | 'download:failed'
    | 'download:done'
  file: DownloadAppFile | UploadAppFile
}>()

const isUpload = computed(() => {
  return props.type.startsWith('upload') && Array.isArray(props.file.uploaded_chunks)
})

const isDownload = computed(() => {
  return props.type.startsWith('download') && (props.file as DownloadAppFile).downloadedBytes
})

/**
 * Size of any kind of file
 */
const size = computed(() => {
  return formatSize(props.file.size || 0)
})

/**
 * Amount uploaded
 */
const uploaded = computed(() => {
  if (!Array.isArray(props.file.uploaded_chunks)) {
    return formatSize(props.file.size || 0)
  }

  const stored = props.file.uploaded_chunks?.length || 0

  if (!stored) {
    return '0 B'
  }

  const percentage = stored / props.file.chunks
  const size = props.file.size || 0

  return formatSize(percentage * size)
})

/**
 * Percentage of any kind of progress
 */
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

/**
 * Speed of any kind of progress
 */
const speed = computed(() => {
  if (isUpload.value) {
    if (!(props.file as UploadAppFile).started_upload_at) {
      return '0 B/s'
    }

    const started = localDateFromUtcString((props.file as UploadAppFile).started_upload_at)

    const seconds = (Date.now().valueOf() - started.valueOf()) / 1000

    const stored = props.file.uploaded_chunks?.length || 0

    if (!stored) {
      return '0 B/s'
    }

    const percentage = stored / props.file.chunks
    const size = props.file.size || 0
    const uploaded = percentage * size

    return formatSize(uploaded / seconds) + '/s'
  } else {
    if (!(props.file as DownloadAppFile).started_download_at) {
      return '0 B/s'
    }

    const started = localDateFromUtcString((props.file as DownloadAppFile).started_download_at)

    const seconds = (Date.now().valueOf() - started.valueOf()) / 1000

    const downloadedBytes = (props.file as DownloadAppFile).downloadedBytes || 0

    if (!downloadedBytes || !props.file.size) {
      return '0 B/s'
    }

    const percentage = downloadedBytes / props.file.size
    const size = props.file.size || 0
    const uploaded = percentage * size

    return formatSize(uploaded / seconds) + '/s'
  }
})
</script>

<template>
  <tr v-if="isUpload || isDownload">
    <td
      class="px-4 py-2 text-gray-700 dark:text-white text-sm truncate"
      :title="file.metadata?.name"
    >
      {{ file.metadata?.name }}
    </td>

    <td class="px-4 py-2 text-gray-700 dark:text-white text-sm truncate" :title="size">
      {{ size }}
    </td>

    <td class="px-4 py-2 text-gray-700 dark:text-white text-sm truncate" :title="uploaded">
      <span v-if="props.type !== 'upload:uploading'">{{ props.type }}</span>
      <span v-else class="text-gray-700 dark:text-gray-400">{{ percentage }}</span>
    </td>

    <td class="px-4 py-2 text-gray-700 dark:text-white text-sm truncate">
      {{ speed }}
    </td>
  </tr>
</template>
