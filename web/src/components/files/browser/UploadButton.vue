<script setup lang="ts">
import { ref, watch } from 'vue'
import type { KeyPair, AppFile } from 'types'

const props = defineProps<{
  dir: AppFile | undefined | null
  kp: KeyPair
  modelValue: boolean
  openFolder: boolean
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
  (event: 'update:openFolder', value: boolean): void
  (event: 'upload-many', files: FileList): void
  (event: 'upload-folder', files: FileList): void
}>()

const input = ref()
const folderInput = ref()

/**
 * Adds selected files to the upload queue
 */
const addFiles = async () => {
  if (input.value && input.value?.files?.length) {
    emits('upload-many', input.value.files)
  }
}

/**
 * Adds selected folder contents to the upload queue
 */
const addFolder = async () => {
  if (folderInput.value?.files?.length) {
    emits('upload-folder', folderInput.value.files)
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

watch(
  () => props.openFolder,
  (value) => {
    if (value === true && folderInput.value) {
      folderInput.value.click()
      emits('update:openFolder', false)
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
  <input
    name="upload-folder-input"
    style="display: none"
    type="file"
    ref="folderInput"
    webkitdirectory
    multiple
    @change="addFolder"
  />
</template>
