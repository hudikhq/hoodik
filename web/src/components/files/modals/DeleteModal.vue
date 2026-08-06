<script setup lang="ts">
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import type { FilesStore, KeyPair, AppFile } from 'types'

const props = defineProps<{
  modelValue: AppFile | undefined
  Storage: FilesStore
  kp: KeyPair
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: AppFile | undefined): void
}>()

/**
 * Confirms removing a single file
 */
const confirmRemove = async () => {
  if (!props.modelValue) return

  await props.Storage.remove(props.kp, props.modelValue)

  emits('update:modelValue', undefined)
}
</script>

<template>
  <CardBoxModal
    :title="$t('common.delete')"
    button="danger"
    :model-value="!!props.modelValue"
    :button-label="$t('files.delete.confirmLabel')"
    :has-cancel="true"
    @cancel="emits('update:modelValue', undefined)"
    @confirm="confirmRemove"
  >
    {{
      props.modelValue?.mime === 'dir'
        ? $t('files.delete.confirmDirectory', { name: props.modelValue?.name })
        : $t('files.delete.confirmFile', { name: props.modelValue?.name })
    }}
  </CardBoxModal>
</template>
