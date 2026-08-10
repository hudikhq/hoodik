<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import type { LinksStore, KeyPair } from 'types'
import { notification, humanizeError } from '!/notify'

const props = defineProps<{
  modelValue: boolean
  Links: LinksStore
  kp: KeyPair
}>()

const { t } = useI18n()

const emits = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
}>()

/**
 * Confirms removing multiple links that were selected
 */
const confirmRemoveAll = async () => {
  try {
    await props.Links.removeAll(props.kp, props.Links.selected)
  } catch (err) {
    notification(t('errors.deleteFailed'), humanizeError(err), 'error')
  }
  emits('update:modelValue', false)
}
</script>

<template>
  <CardBoxModal
    :title="$t('links.deleteModal.title')"
    button="danger"
    :model-value="props.modelValue"
    :button-label="$t('links.deleteModal.confirm')"
    :has-cancel="true"
    @cancel="emits('update:modelValue', false)"
    @confirm="confirmRemoveAll"
  >
    <p>
      {{ $t('links.deleteModal.body', Links.selected.length) }}
    </p>
  </CardBoxModal>
</template>
