<script setup lang="ts">
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import type { FilesStore, KeyPair, AppFile } from 'types'
import { useRouter } from 'vue-router'

const router = useRouter()

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

  router.push({
    name: 'files',
    params: { file_id: props.modelValue.file_id }
  })

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
    Are you sure you want to delete forever '{{ props.modelValue?.name }}'
    <span v-if="props.modelValue?.mime === 'dir'"> directory</span>?
  </CardBoxModal>
</template>
