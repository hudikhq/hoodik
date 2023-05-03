<script setup lang="ts">
import { mdiTrashCan, mdiDownload } from '@mdi/js'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import type { FilesStore, KeyPair, ListAppFile } from 'types'
import { ref, watch } from 'vue'

const props = defineProps<{
  modelValue: ListAppFile | undefined
  hideDelete?: boolean
  storage: FilesStore
  kp: KeyPair
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: ListAppFile | undefined): void
  (event: 'download', file: ListAppFile): void
  (event: 'remove', file: ListAppFile): void
}>()

const imageUrl = ref<string>()

/**
 * Load the file data
 */
const get = async () => {
  if (!props.modelValue) return

  const file = await props.storage.get(props.modelValue, props.kp)

  if (!file || !file.data) return

  const blob = new Blob([file.data])
  imageUrl.value = URL.createObjectURL(blob)

  emits('update:modelValue', file)
}

/**
 * Close and destroy the modal.
 */
const cancel = () => {
  emits('update:modelValue', undefined)
  imageUrl.value = undefined
}

/**
 * Start the download of a file through the
 * regular download process.
 */
const download = () => {
  if (!props.modelValue) return
  emits('download', props.modelValue)
  emits('update:modelValue', undefined)
  imageUrl.value = undefined
}

/**
 * Remove the file from the storage.
 */
const remove = () => {
  if (!props.modelValue) return
  emits('remove', props.modelValue)
  emits('update:modelValue', undefined)
  imageUrl.value = undefined
}

watch(
  () => props.modelValue,
  () => get(),
  { immediate: true }
)
</script>

<template>
  <CardBoxModal
    :model-value="!!props.modelValue"
    :hide-submit="true"
    :has-cancel="false"
    @cancel="cancel"
  >
    <div class="flex justify-center max-h-[95vh] max-w-[95vw]">
      <img
        v-if="imageUrl"
        :src="imageUrl"
        :alt="props.modelValue?.metadata?.name"
        class="max-h-[95vh] max-w-[95vw]"
      />
      <img
        v-else
        :src="props.modelValue?.metadata?.thumbnail"
        :alt="props.modelValue?.metadata?.name"
      />
    </div>
    <div class="flex justify-center p-3" v-if="props.modelValue">
      <BaseButton color="lightDark" :icon="mdiDownload" small @click="download" label="Download" />
      <BaseButton
        label="Delete"
        v-if="!hideDelete"
        color="danger"
        :icon="mdiTrashCan"
        small
        class="ml-2"
        @click="remove"
      />
    </div>
  </CardBoxModal>
</template>
