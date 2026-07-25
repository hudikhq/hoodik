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
    :title="$t('files.delete.selectedTitle')"
    button="danger"
    :model-value="props.modelValue"
    :button-label="$t('files.delete.confirmLabel')"
    :has-cancel="true"
    @cancel="emits('update:modelValue', false)"
    @confirm="confirmRemoveAll"
  >
    <template v-if="Storage.selected && Storage.selected.length > 1">
      <p>{{ $t('files.delete.confirmMany', { count: Storage.selected.length }) }}</p>
    </template>

    <template v-else v-for="file in Storage.selected" :key="file.id">
      <p>
        {{
          file?.mime === 'dir'
            ? $t('files.delete.confirmDirectory', { name: file?.name })
            : $t('files.delete.confirmFile', { name: file?.name })
        }}
      </p>
    </template>
  </CardBoxModal>
</template>
