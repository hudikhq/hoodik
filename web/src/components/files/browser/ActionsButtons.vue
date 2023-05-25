<script setup lang="ts">
import PureButton from '@/components/ui/PureButton.vue'
import { mdiTrashCan, mdiEye, mdiDownload, mdiLink } from '@mdi/js'
import type { ListAppFile } from 'types'
import { computed } from 'vue'

const props = defineProps<{
  modelValue: ListAppFile
  hideDelete?: boolean
  share?: boolean
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: ListAppFile | undefined): void
  (event: 'details', file: ListAppFile): void
  (event: 'remove', file: ListAppFile): void
  (event: 'link', file: ListAppFile): void
  (event: 'download', file: ListAppFile): void
}>()

const file = computed(() => props.modelValue)

const hasPreview = computed(() => {
  return file.value?.metadata?.thumbnail && file.value?.finished_upload_at
})

const hasDownload = computed(() => {
  return file.value?.mime !== 'dir' && file.value?.finished_upload_at
})

const canHaveALink = computed(() => {
  return file.value?.mime !== 'dir' && file.value?.finished_upload_at && props.share
})
</script>

<template>
  <PureButton
    :icon="mdiEye"
    @click="emits('details', file)"
    label="Details"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    v-if="hasPreview"
    :icon="mdiEye"
    :to="{
      name: 'file-preview',
      params: { file_id: file.id }
    }"
    label="Preview"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    :icon="mdiDownload"
    @click="emits('download', file)"
    v-if="hasDownload"
    label="Download"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    :icon="mdiLink"
    @click="emits('link', file)"
    v-if="canHaveALink"
    label="Public link"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    v-if="!props.hideDelete"
    :icon="mdiTrashCan"
    @click="emits('remove', file)"
    label="Delete"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />
</template>
