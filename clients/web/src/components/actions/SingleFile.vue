<script setup lang="ts">
import { formatSize, localDateFromUtcString } from '@/stores'
import type { UploadAppFile } from '@/stores/storage/types'
import { computed } from 'vue'

const props = defineProps<{
  type: 'upload' | 'waiting' | 'failed' | 'done'
  file: UploadAppFile
}>()

const uploaded = computed(() => {
  const stored = props.file.uploaded_chunks?.length || 0

  if (!stored) {
    return '0 B'
  }

  const percentage = stored / props.file.chunks
  const size = props.file.size || props.file.file.size || 0

  return formatSize(percentage * size)
})

const total = computed(() => {
  return formatSize(props.file.size || 0)
})

const speed = computed(() => {
  if (!props.file.started_upload_at) {
    return '0 B/s'
  }

  const started = localDateFromUtcString(props.file.started_upload_at)

  const seconds = (Date.now().valueOf() - started.valueOf()) / 1000

  const stored = props.file.uploaded_chunks?.length || 0

  if (!stored) {
    return '0 B/s'
  }

  const percentage = stored / props.file.chunks
  const size = props.file.size || props.file.file.size || 0
  const uploaded = percentage * size

  return formatSize(uploaded / seconds) + '/s'
})
</script>

<template>
  <p class="block text-xs text-green-900 dark:text-green-400" v-if="props.type === 'upload'">
    {{ file.metadata?.name }} {{ uploaded }} of {{ total }} ({{ speed }})
  </p>
  <p class="block text-xs text-gray-900 dark:text-gray-400" v-else-if="props.type === 'waiting'">
    Waiting to start uploading: {{ file.metadata?.name }}
  </p>
  <p class="block text-xs text-red-900 dark:text-red-400" v-else-if="props.type === 'failed'">
    Failed uploading {{ file.metadata?.name }} at {{ uploaded }} of {{ total }}
  </p>
  <p class="block text-xs text-gray-900 dark:text-gray-400" v-else-if="props.type === 'done'">
    Done uploading {{ file.metadata?.name }}
  </p>
</template>
