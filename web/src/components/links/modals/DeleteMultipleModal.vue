<script setup lang="ts">
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import type { LinksStore, KeyPair } from 'types'

const props = defineProps<{
  modelValue: boolean
  Links: LinksStore
  kp: KeyPair
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
}>()

/**
 * Confirms removing multiple links that were selected
 */
const confirmRemoveAll = async () => {
  await props.Links.removeAll(props.kp, props.Links.selected)
  emits('update:modelValue', false)
}
</script>

<template>
  <CardBoxModal
    title="Delete selected links"
    button="danger"
    :model-value="props.modelValue"
    button-label="Yes, delete"
    :has-cancel="true"
    @cancel="emits('update:modelValue', false)"
    @confirm="confirmRemoveAll"
  >
    <p>
      Are you sure you want to delete
      {{ Links.selected.length }} {{ Links.selected.length == 1 ? 'item' : 'items' }}?
    </p>
  </CardBoxModal>
</template>
