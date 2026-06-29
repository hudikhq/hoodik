<script setup lang="ts">
import ActionsButtons from '@/components/files/browser/ActionsButtons.vue'
import DropdownModal from '@/components/ui/DropdownModal.vue'
import type { AppFile } from 'types'
import { computed } from 'vue'

const props = defineProps<{
  modelValue: AppFile | undefined
  hideDelete?: boolean
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: AppFile | undefined): void
  (event: 'remove', file: AppFile): void
  (event: 'sharing', file: AppFile): void
  (event: 'download', file: AppFile): void
  (event: 'details', file: AppFile): void
  (event: 'rename', file: AppFile): void
  (event: 'fork', file: AppFile): void
  (event: 'leave', file: AppFile): void
}>()

const file = computed(() => props.modelValue)
</script>

<template>
  <DropdownModal :model-value="!!file" @cancel="emits('update:modelValue', undefined)">
    <ActionsButtons
      v-if="file"
      :model-value="file"
      :hide-delete="props.hideDelete"
      @remove="emits('remove', file)"
      @details="emits('details', file)"
      @download="emits('download', file)"
      @rename="emits('rename', file)"
      @sharing="emits('sharing', file)"
      @fork="emits('fork', file)"
      @leave="emits('leave', file)"
    />
  </DropdownModal>
</template>
