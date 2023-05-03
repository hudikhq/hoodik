<script setup lang="ts">
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import type { FilesStore, KeyPair, ListAppFile } from 'types'

const props = defineProps<{
  modelValue: ListAppFile | undefined
  storage: FilesStore
  kp: KeyPair
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: ListAppFile | undefined): void
}>()

/**
 * Confirms removing a single file
 */
const confirmRemove = async () => {
  if (!props.modelValue) return

  await props.storage.remove(props.kp, props.modelValue)

  emits('update:modelValue', undefined)
}
</script>

<template>
  <CardBoxModal
    title="Delete"
    button="danger"
    :model-value="!!props.modelValue"
    button-label="Yes, delete"
    :has-cancel="true"
    @cancel="emits('update:modelValue', undefined)"
    @confirm="confirmRemove"
  >
    Are you sure you want to delete forever '{{ props.modelValue?.metadata?.name }}'
    <span v-if="props.modelValue?.mime === 'dir'"> directory</span>?
  </CardBoxModal>
</template>
