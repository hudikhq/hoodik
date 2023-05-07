<script setup lang="ts">
import PureButton from '@/components/ui/PureButton.vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import { mdiTrashCan, mdiEye, mdiDownload } from '@mdi/js'
import type { ListAppFile } from 'types'
import { computed } from 'vue'

const props = defineProps<{
  modelValue: ListAppFile | undefined
  hideDelete?: boolean
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: ListAppFile | undefined): void
  (event: 'remove', file: ListAppFile): void
  (event: 'preview', file: ListAppFile): void
  (event: 'download', file: ListAppFile): void
}>()

const file = computed(() => props.modelValue)

const hasPreview = computed(() => {
  return file.value?.metadata?.thumbnail && file.value?.finished_upload_at
})

const hasDownload = computed(() => {
  return file.value?.mime !== 'dir' && file.value?.finished_upload_at
})
</script>

<template>
  <CardBoxModal
    :model-value="!!file"
    :has-cancel="false"
    :hide-submit="true"
    @cancel="emits('update:modelValue', undefined)"
  >
    <div class="" v-if="file">
      <PureButton
        v-if="hasPreview"
        :icon="mdiEye"
        @click="emits('preview', file)"
        label="Preview"
        class="block text-left border-b-[1px] border-t-[1px] border-brownish-800 w-full"
      />

      <PureButton
        :icon="mdiDownload"
        @click="emits('download', file)"
        v-if="hasDownload"
        label="Download"
        :class="{
          'block text-left border-b-[1px] border-brownish-800 w-full': true,
          'border-t-[1px]': !hasPreview
        }"
      />

      <PureButton
        v-if="!props.hideDelete"
        :icon="mdiTrashCan"
        @click="emits('remove', file)"
        label="Delete"
        :class="{
          'block text-left border-b-[1px] border-brownish-800 w-full': true,
          'border-t-[1px]': !hasDownload && !hasPreview
        }"
      />
    </div>
  </CardBoxModal>
</template>
