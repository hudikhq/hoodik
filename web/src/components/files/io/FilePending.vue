<script setup lang="ts">
import BaseIcon from '@/components/ui/BaseIcon.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import {
  mdiClose,
  mdiTimerSandEmpty,
  mdiArrowUpBoldOutline,
  mdiArrowDownBoldOutline
} from '@mdi/js'
import type { DownloadAppFile, UploadAppFile, QueueItemActionType } from 'types'

const props = defineProps<{
  type: QueueItemActionType
  file: DownloadAppFile | UploadAppFile
  name: string
  size: string
  isUpload: boolean
}>()

const emits = defineEmits(['remove'])
</script>

<template>
  <div class="py-2 pl-2 w-24 text-left">
    <BaseButton
      title="Remove from the list"
      :icon="mdiClose"
      small
      color="light"
      :noBorder="true"
      :outline="false"
      @click="emits('remove', file, props.type)"
    />
    <BaseIcon v-if="props.isUpload" :path="mdiArrowDownBoldOutline" h="h-5" w="w-5" />
    <BaseIcon v-else :path="mdiArrowUpBoldOutline" h="h-5" w="w-5" />
    <BaseIcon :path="mdiTimerSandEmpty" h="h-5" w="w-5" />
  </div>
  <div class="py-2 flex-1 text-left inline-block truncate">
    {{ name }}
  </div>
  <div class="p-2 w-22 text-right">
    {{ size }}
  </div>
</template>
