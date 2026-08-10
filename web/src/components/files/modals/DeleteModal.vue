<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import type { FilesStore, KeyPair, AppFile } from 'types'
import { notification, humanizeError } from '!/notify'

const props = defineProps<{
  modelValue: AppFile | undefined
  Storage: FilesStore
  kp: KeyPair
}>()

const { t } = useI18n()

const emits = defineEmits<{
  (event: 'update:modelValue', value: AppFile | undefined): void
}>()

/**
 * Confirms removing a single file
 */
const confirmRemove = async () => {
  if (!props.modelValue) return

  // The dialog is already closing by the time this runs, so a failure has
  // to announce itself — the row simply staying put reads as success.
  try {
    await props.Storage.remove(props.kp, props.modelValue)
  } catch (err) {
    notification(t('errors.deleteFailed'), humanizeError(err), 'error')
  }

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
