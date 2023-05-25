<script setup lang="ts">
import ActionsButtons from '@/components/files/browser/ActionsButtons.vue'
import DropdownModal from '@/components/ui/DropdownModal.vue'
import type { ListAppFile } from 'types'
import { computed } from 'vue'

const props = defineProps<{
  modelValue: ListAppFile | undefined
  hideDelete?: boolean
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: ListAppFile | undefined): void
  (event: 'remove', file: ListAppFile): void
  (event: 'download', file: ListAppFile): void
  (event: 'details', file: ListAppFile): void
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
    />
  </DropdownModal>
</template>
