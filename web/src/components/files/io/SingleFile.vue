<script setup lang="ts">
import { formatSize } from '!'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import SpinnerIcon from '@/components/ui/SpinnerIcon.vue'
import { computed } from 'vue'
import {
  mdiClose,
  mdiCheck,
  mdiCancel,
  mdiTimerSandEmpty,
  mdiArrowUpBoldOutline,
  mdiArrowDownBoldOutline
} from '@mdi/js'
import type { DownloadAppFile, UploadAppFile, QueueItemActionType } from 'types'

const props = defineProps<{
  type: QueueItemActionType
  file: DownloadAppFile | UploadAppFile
}>()

const emits = defineEmits(['remove'])

const isUpload = computed(() => props.type.startsWith('upload'))

const size = computed(() => formatSize(props.file.size || 0))

const name = computed<string>(() => `${props.file.name || props.file.id}`)

const error = computed(() => {
  if (!props.file.error) return ''
  if (props.file.error.context && typeof props.file.error.context === 'string') {
    return props.file.error.context
  }
  if (props.file.error.context) return 'Something went wrong'
  return `${props.file.error}`
})

const titleText = computed(() => {
  if (error.value) return error.value
  return `${props.type.split(':')[0]}: ${name.value} (${size.value})`
})

const percentage = computed(() => {
  if (isUpload.value) {
    const stored = props.file.uploaded_chunks?.length || 0
    if (!stored) return '0%'
    return Math.round((stored / props.file.chunks) * 100) + '%'
  }
  const downloadedBytes = (props.file as DownloadAppFile).downloadedBytes || 0
  if (!props.file.size) return '0%'
  return Math.round((downloadedBytes / props.file.size) * 100) + '%'
})

const speed = computed(() => {
  if (!props.type.endsWith('running')) return ''

  if (isUpload.value) {
    const started = (props.file as UploadAppFile).started_upload_at
    if (!started) return '0 B/s'
    const seconds = (Date.now() - new Date(started).valueOf()) / 1000
    const stored = props.file.uploaded_chunks?.length || 0
    if (!stored) return 'Preparing...'
    const uploaded = (stored / props.file.chunks) * (props.file.size || 0)
    return formatSize(uploaded / seconds) + '/s'
  } else {
    const started = (props.file as DownloadAppFile).started_download_at
    if (!started) return '0 B/s'
    const seconds = (Date.now() - new Date(started).valueOf()) / 1000
    const downloadedBytes = (props.file as DownloadAppFile).downloadedBytes || 0
    if (!downloadedBytes || !props.file.size) return '0 B/s'
    return formatSize(downloadedBytes / seconds) + '/s'
  }
})

const eta = computed(() => {
  if (!props.type.endsWith('running')) return ''

  if (isUpload.value) {
    const stored = props.file.uploaded_chunks?.length || 0
    const started = (props.file as UploadAppFile).started_upload_at
    if (!stored || !props.file.chunks || !started) return ''
    const elapsed = (Date.now() - new Date(started).valueOf()) / 1000
    const rate = stored / elapsed
    const remaining = props.file.chunks - stored
    const secs = Math.round(remaining / rate)
    if (!isFinite(secs) || secs <= 0) return ''
    return secs < 60 ? `~${secs}s` : `~${Math.round(secs / 60)}m`
  } else {
    const dlBytes = (props.file as DownloadAppFile).downloadedBytes || 0
    const started = (props.file as DownloadAppFile).started_download_at
    if (!dlBytes || !props.file.size || !started) return ''
    const elapsed = (Date.now() - new Date(started).valueOf()) / 1000
    const rate = dlBytes / elapsed
    const remaining = props.file.size - dlBytes
    const secs = Math.round(remaining / rate)
    if (!isFinite(secs) || secs <= 0) return ''
    return secs < 60 ? `~${secs}s` : `~${Math.round(secs / 60)}m`
  }
})

const accentClass = computed(() => {
  if (props.type.endsWith('running')) return 'border-greeny-600 dark:border-greeny-400'
  if (props.type.endsWith('done')) return 'border-greeny-800 dark:border-greeny-500'
  if (props.type.endsWith('failed')) return 'border-redish-500 dark:border-redish-400'
  return 'border-brownish-400 dark:border-brownish-500'
})

const progressBarColor = computed(() => {
  if (props.type.endsWith('failed')) return 'bg-redish-500'
  return 'bg-greeny-600 dark:bg-greeny-400'
})

const showProgress = computed(
  () => props.type.endsWith('running') || props.type.endsWith('waiting')
)
</script>

<template>
  <div :title="titleText" class="border-l-4 px-3 py-2 min-w-0" :class="accentClass">
    <!-- Row 1: dismiss + direction + state icon | filename | right meta -->
    <div class="flex items-center gap-2 min-w-0">
      <!-- Left icon cluster — shrink-0 so icons never compress -->
      <div class="flex items-center shrink-0 -ml-1">
        <button
          class="min-h-[36px] min-w-[36px] flex items-center justify-center rounded hover:bg-brownish-200 dark:hover:bg-brownish-700 transition-colors"
          :title="props.type.endsWith('running') ? 'Cancel transfer' : 'Remove from list'"
          @click="emits('remove', file, props.type)"
        >
          <BaseIcon :path="mdiClose" h="h-4" w="w-4" />
        </button>
        <BaseIcon
          :path="isUpload ? mdiArrowUpBoldOutline : mdiArrowDownBoldOutline"
          h="h-4"
          w="w-4"
        />
        <SpinnerIcon v-if="props.type.endsWith('running')" h="h-4" w="w-4" />
        <BaseIcon v-else-if="props.type.endsWith('done')" :path="mdiCheck" h="h-4" w="w-4" />
        <BaseIcon v-else-if="props.type.endsWith('failed')" :path="mdiCancel" h="h-4" w="w-4" />
        <BaseIcon v-else :path="mdiTimerSandEmpty" h="h-4" w="w-4" />
      </div>

      <!-- Middle: filename + optional error line — flex-1 min-w-0 required for truncate in flex -->
      <div class="flex-1 min-w-0">
        <div class="text-sm truncate">{{ name }}</div>
        <div
          v-if="props.type.endsWith('failed') && error"
          class="text-xs text-redish-800 dark:text-redish-400 truncate"
        >
          {{ error }}
        </div>
      </div>

      <!-- Right: speed+ETA for running, size for others -->
      <div class="shrink-0 text-xs text-right tabular-nums text-brownish-600 dark:text-brownish-300">
        <template v-if="props.type.endsWith('running')">
          {{ speed }}<span v-if="eta" class="ml-1 opacity-60 hidden sm:inline">{{ eta }}</span>
        </template>
        <template v-else>
          {{ size }}
        </template>
      </div>
    </div>

    <!-- Row 2: progress track — full width, smooth animated fill -->
    <div
      v-if="showProgress"
      class="mt-1.5 h-1.5 rounded-full bg-brownish-200 dark:bg-brownish-700 overflow-hidden"
    >
      <div
        class="h-full rounded-full transition-[width] duration-500 ease-out"
        :class="progressBarColor"
        :style="{ width: percentage }"
      />
    </div>
  </div>
</template>
