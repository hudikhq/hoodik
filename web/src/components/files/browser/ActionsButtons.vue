<script setup lang="ts">
import PureButton from '@/components/ui/PureButton.vue'
import { mdiTrashCan, mdiEye, mdiDownload, mdiLink, mdiPencil } from '@mdi/js'
import type { AppFile } from 'types'
import { computed } from 'vue'

const props = defineProps<{
  modelValue: AppFile
  hideDelete?: boolean
  share?: boolean
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: AppFile | undefined): void
  (event: 'details', file: AppFile): void
  (event: 'remove', file: AppFile): void
  (event: 'link', file: AppFile): void
  (event: 'rename', file: AppFile): void
  (event: 'download', file: AppFile): void
}>()

const file = computed(() => props.modelValue)

const hasPreview = computed(() => {
  return file.value?.thumbnail && file.value?.finished_upload_at
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
    name="details"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    v-if="hasPreview"
    :icon="mdiEye"
    :to="{
      name: 'file-preview',
      params: { id: file.id }
    }"
    label="Preview"
    name="preview"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    :icon="mdiDownload"
    @click="emits('download', file)"
    v-if="hasDownload"
    label="Download"
    name="download"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    :icon="mdiLink"
    @click="emits('link', file)"
    v-if="canHaveALink"
    label="Public link"
    name="public-link"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    v-if="!props.hideDelete"
    :icon="mdiTrashCan"
    @click="emits('remove', file)"
    label="Delete"
    name="delete"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />

  <PureButton
    v-if="file.is_owner && (file.finished_upload_at || file.mime === 'dir')"
    :icon="mdiPencil"
    @click="emits('rename', file)"
    label="Rename"
    name="rename"
    class="block text-left p-2 sm:p-0 border-brownish-800 w-full hover:bg-brownish-600"
  />
</template>
