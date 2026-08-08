<script setup lang="ts">
import { mdiAlertCircleOutline } from '@mdi/js'
import BaseIcon from '@/components/ui/BaseIcon.vue'
</script>

<template>
  <div
    role="alert"
    class="flex items-start gap-2 rounded-md border border-redish-500/30 bg-redish-500/10 px-3 py-2 mb-3 text-sm text-redish-700 dark:border-redish-400/25 dark:bg-redish-400/10 dark:text-redish-200"
  >
    <BaseIcon :path="mdiAlertCircleOutline" :size="16" w="w-4" h="h-4" class="mt-0.5 shrink-0" />
    <span><slot /></span>
  </div>
</template>
