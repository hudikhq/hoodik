<script setup lang="ts">
import { store as uploadStore } from '!/storage/upload'
import { ref, watch } from 'vue'
import type { KeyPair, ListAppFile } from 'types'

const props = defineProps<{
  dir: ListAppFile | undefined | null
  kp: KeyPair
  modelValue: boolean
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
}>()

const upload = uploadStore()
const input = ref()
/**
 * Adds selected files to the upload queue
 */
const addFiles = async () => {
  if (input.value && input.value?.files?.length) {
    for (let i = 0; i < input.value?.files?.length; i++) {
      try {
        await upload.push(props.kp, input.value?.files?.[i], props.dir?.id || undefined)
      } catch (error) {
        // TODO: Add some kind of notifications store...
      }
    }
  }

  if (input.value) {
    input.value.value = ''
  }

  if (!upload.active) {
    upload.active = true
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
  <input style="display: none" type="file" ref="input" multiple @change="addFiles" />
</template>
