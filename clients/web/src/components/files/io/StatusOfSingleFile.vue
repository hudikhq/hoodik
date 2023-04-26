<script setup lang="ts">
import { formatSize, localDateFromUtcString } from '@/stores'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { computed } from 'vue'
import {
  mdiClose,
  mdiSkullCrossbones,
  mdiCloudLockOpenOutline,
  mdiTimerSandEmpty,
  mdiArrowUpBoldOutline,
  mdiArrowDownBoldOutline
} from '@mdi/js'
import type { DownloadAppFile, UploadAppFile, QueueItemActionType } from '@/stores/types'

const props = defineProps<{
  type: QueueItemActionType
  file: DownloadAppFile | UploadAppFile
}>()

const emits = defineEmits(['remove'])

const isUpload = computed(() => {
  return props.type.startsWith('upload')
})

const isDownload = computed(() => {
  return props.type.startsWith('download')
})

/**
 * Size of any kind of file
 */
const size = computed(() => {
  return formatSize(props.file.size || 0)
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
  if (!props.file.metadata?.name) {
    return `${props.file.id}`
  }

  return `${props.file.metadata?.name || props.file.id}`
})

const nameShort = computed(() => {
  let n = name.value

  if (n.length > 20) {
    n = n.substring(0, 20) + '...'
  }

  return n
})

const errorShort = computed(() => {
  let e = error.value

  if (e.length > 40) {
    e = e.substring(0, 40) + '...'
  }

  return e
})

const titleText = computed(() => {
  if (error.value) {
    return error.value
  }

  return `${name.value} (${size.value})`
})
</script>

<template>
  <tr
    v-if="isUpload || isDownload"
    :class="{
      'text-brownish-500 w-full': true,
      'text-brownish-900 dark:text-brownish-200': props.type.endsWith('running'),
      'text-red-900 dark:text-red-200': props.type.endsWith('failed'),
      'text-green-900 dark:text-green-200': props.type.endsWith('done')
    }"
  >
    <td :title="titleText" class="text-sm">
      <BaseButton
        v-if="props.type === 'download:running'"
        :icon="mdiClose"
        small
        color="lightDark"
        :noBorder="true"
        :outline="false"
        class="float-left p-0"
        @click="emits('remove', file, props.type)"
        :disabled="true"
      />
      <BaseButton
        v-else
        :icon="mdiClose"
        small
        color="lightDark"
        :noBorder="true"
        :outline="false"
        class="float-left p-0"
        @click="emits('remove', file, props.type)"
      />

      <span class="float-left">
        <BaseIcon
          v-if="props.type === 'upload:running'"
          :path="mdiArrowUpBoldOutline"
          class="m-1 p-0"
          h="h-5"
          w="w-5"
        />

        <BaseIcon
          v-if="props.type === 'upload:waiting'"
          :path="mdiTimerSandEmpty"
          class="m-1 p-0"
          h="h-5"
          w="w-5"
        />

        <BaseIcon
          v-if="props.type === 'upload:done'"
          :path="mdiArrowUpBoldOutline"
          class="m-1 p-0"
          h="h-5"
          w="w-5"
        />
        <BaseIcon
          v-if="props.type === 'upload:failed'"
          :path="mdiSkullCrossbones"
          class="m-1 p-0"
          h="h-5"
          w="w-5"
        />
        <BaseIcon
          v-if="props.type === 'download:running'"
          :path="mdiArrowDownBoldOutline"
          class="m-1 p-0"
          h="h-5"
          w="w-5"
        />
        <BaseIcon
          v-if="props.type === 'download:waiting'"
          :path="mdiTimerSandEmpty"
          class="m-1 p-0"
          h="h-5"
          w="w-5"
        />
        <BaseIcon
          v-if="props.type === 'download:done'"
          :path="mdiCloudLockOpenOutline"
          class="m-1 p-0"
          h="h-5"
          w="w-5"
        />
        <BaseIcon
          v-if="props.type === 'download:failed'"
          :path="mdiSkullCrossbones"
          class="m-1 p-0"
          h="h-5"
          w="w-5"
        />
      </span>

      <span class="float-left mt-1">
        {{ nameShort }}
      </span>

      <span class="float-left mt-1 ml-2 text-brownish-700 dark:text-brownish-400 xs:hidden">
        ({{ size }})
      </span>
      <span
        class="float-left mt-1 ml-2 text-red-700 dark:text-red-400"
        v-if="props.type.endsWith('failed')"
      >
        {{ errorShort }}
      </span>

      <span
        v-if="props.type.endsWith('running') || props.type.endsWith('done')"
        class="float-right mt-1 ml-2"
      >
        {{ speed }}
      </span>

      <div
        :class="{
          'absolute bottom-0 py-0.5': true,
          'bg-green-200 dark:bg-green-900': props.type.endsWith('running')
        }"
        :style="{
          width: percentage
        }"
      ></div>
    </td>
  </tr>
</template>
