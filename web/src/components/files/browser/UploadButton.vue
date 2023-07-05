<script setup lang="ts">
import { ref, watch } from 'vue'
import type { KeyPair, AppFile } from 'types'

const props = defineProps<{
  dir: AppFile | undefined | null
  kp: KeyPair
  modelValue: boolean
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
  (event: 'upload-many', files: FileList): void
}>()

const input = ref()
/**
 * Adds selected files to the upload queue
 */
const addFiles = async () => {
  if (input.value && input.value?.files?.length) {
    emits('upload-many', input.value.files)
  }
}

watch(
  () => props.modelValue,
  (value) => {
    if (value === true && input.value) {
      input.value.click()
      emits('update:modelValue', false)
    }
  }
)
</script>
<template>
  <input
    name="upload-file-input"
    style="display: none"
    type="file"
    ref="input"
    multiple
    @change="addFiles"
  />
</template>
