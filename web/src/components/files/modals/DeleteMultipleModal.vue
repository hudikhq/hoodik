<script setup lang="ts">
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import type { FilesStore, KeyPair } from 'types'

const props = defineProps<{
  modelValue: boolean
  Storage: FilesStore
  kp: KeyPair
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
}>()

/**
 * Confirms removing multiple files that were selected
 */
const confirmRemoveAll = async () => {
  await props.Storage.removeAll(props.kp, props.Storage.selected)
  emits('update:modelValue', false)
}
</script>

<template>
  <CardBoxModal
    title="Delete selected"
    button="danger"
    :model-value="props.modelValue"
    button-label="Yes, delete"
    :has-cancel="true"
    @cancel="emits('update:modelValue', false)"
    @confirm="confirmRemoveAll"
  >
    <template v-if="Storage.selected && Storage.selected.length > 1">
      <p>Are you sure you want to delete {{ Storage.selected.length }} items?</p>
    </template>

    <template v-else v-for="file in Storage.selected" :key="file.id">
      <p>
        Are you sure you want to delete forever '{{ file?.name }}'
        <span v-if="file?.mime === 'dir'"> directory</span>
        ?
      </p>
    </template>
  </CardBoxModal>
</template>
