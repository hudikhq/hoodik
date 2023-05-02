<script setup lang="ts">
import PureButton from '@/components/ui/PureButton.vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import { mdiTrashCan, mdiEye, mdiDownload } from '@mdi/js'
import type { ListAppFile } from '@/types'

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
</script>

<template>
  <CardBoxModal
    :model-value="!!props.modelValue"
    :has-cancel="false"
    :hide-submit="true"
    @cancel="emits('update:modelValue', undefined)"
  >
    <div class="" v-if="props.modelValue">
      <PureButton
        v-if="props.modelValue.metadata?.thumbnail"
        :icon="mdiEye"
        @click="emits('preview', props.modelValue)"
        label="Preview"
        class="block text-left border-b-[1px] border-t-[1px] border-brownish-800 w-full"
      />

      <PureButton
        :icon="mdiDownload"
        @click="emits('download', props.modelValue)"
        :disabled="props.modelValue.mime === 'dir'"
        label="Download"
        :class="{
          'block text-left border-b-[1px] border-brownish-800 w-full': true,
          'border-t-[1px]': !props.modelValue.metadata?.thumbnail
        }"
      />

      <PureButton
        v-if="!props.hideDelete"
        :icon="mdiTrashCan"
        @click="emits('remove', props.modelValue)"
        label="Delete"
        class="block text-left border-b-[1px] border-brownish-800 w-full"
      />
    </div>
  </CardBoxModal>
</template>
