<script setup lang="ts">
import { formatSize, localDateFromUtcString } from '!'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import SpinnerIcon from '@/components/ui/SpinnerIcon.vue'
import { computed } from 'vue'
import { mdiClose, mdiArrowUpBoldOutline, mdiArrowDownBoldOutline } from '@mdi/js'
import type { DownloadAppFile, UploadAppFile, QueueItemActionType } from 'types'

const props = defineProps<{
  type: QueueItemActionType
  file: DownloadAppFile | UploadAppFile
  name: string
  size: string
  isUpload: boolean
}>()

const emits = defineEmits(['remove'])

const speed = computed(() => {
  if (props.isUpload) {
    if (!(props.file as UploadAppFile).started_upload_at) {
      return '0 B/s'
    }

    const started = new Date((props.file as UploadAppFile).started_upload_at as string)

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

    const started = new Date((props.file as DownloadAppFile).started_download_at as string)

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
  <div class="py-2 pl-2 w-24 text-left">
    <BaseButton
      title="Cancel the transfer"
      :icon="mdiClose"
      small
      color="light"
      :noBorder="true"
      :outline="false"
      @click="emits('remove', file, props.type)"
    />
    <BaseIcon v-if="props.isUpload" :path="mdiArrowDownBoldOutline" h="h-5" w="w-5" />
    <BaseIcon v-else :path="mdiArrowUpBoldOutline" h="h-5" w="w-5" />
    <SpinnerIcon h="h-5" w="w-5" />
  </div>
  <div class="py-2 flex-1 text-left inline-block truncate">
    {{ props.name }}
  </div>
  <div class="p-2 w-22 text-right">
    {{ props.size }}
  </div>
  <div class="p-2 w-28 text-right">{{ speed }}</div>
</template>
