<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue'
import type { Preview } from '!/preview'

const props = defineProps<{
  modelValue: Preview
}>()

const blobUrl = ref<string>()

async function load() {
  if (blobUrl.value) {
    URL.revokeObjectURL(blobUrl.value)
    blobUrl.value = undefined
  }
  const buffer = (await props.modelValue.load()).buffer
  const blob = new Blob([buffer], { type: 'application/pdf' })
  blobUrl.value = URL.createObjectURL(blob)
}

watch(() => props.modelValue, load, { immediate: true })

onUnmounted(() => {
  if (blobUrl.value) {
    URL.revokeObjectURL(blobUrl.value)
  }
})
</script>

<template>
  <iframe
    v-if="blobUrl"
    :src="blobUrl"
    style="width: 100%; flex: 1; min-height: 0; border: none"
  />
</template>
