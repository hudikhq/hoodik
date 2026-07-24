<script setup lang="ts">
import BaseButton from '@/components/ui/BaseButton.vue'
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import * as index from '!'
import { mdiMagnify } from '@mdi/js'

const { t } = useI18n()

const emits = defineEmits<{
  (event: 'search'): void
}>()

const label = computed(() => {
  const os = index.os()

  let label = t('common.search')

  if (os === 'macos') {
    label += ' (⌘K)'
  } else if (os === 'windows' || os === 'linux') {
    label += ' (Ctrl+K)'
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
    color="light"
    :small="true"
    :label="label"
    @click="emits('search')"
  />
</template>
