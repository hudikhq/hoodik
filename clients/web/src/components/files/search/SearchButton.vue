<script setup lang="ts">
import BaseButton from '@/components/ui/BaseButton.vue'
import { computed, onMounted } from 'vue'
import * as index from '@/stores'
import { mdiMagnify } from '@mdi/js'

const emits = defineEmits<{
  (event: 'search'): void
}>()

const label = computed(() => {
  const os = index.os()

  let label = 'Search'

  if (os === 'macos') {
    label += ' (⌘ + K)'
  } else if (os === 'windows' || os === 'linux') {
    label += ' (Ctrl + K)'
  }

  return label
})

const fieldFocusHook = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
    e.preventDefault()

    emits('search')
  }
}

onMounted(() => {
  window.addEventListener('keydown', fieldFocusHook)
})
</script>
<template>
  <BaseButton
    :icon="mdiMagnify"
    color="lightDark"
    :small="true"
    :label="label"
    @click="emits('search')"
  />
</template>
