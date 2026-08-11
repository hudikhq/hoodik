<script setup lang="ts">
import { mdiFileDocumentOutline } from '@mdi/js'
import BaseIcon from '@/components/ui/BaseIcon.vue'
</script>

<template>
  <div class="flex flex-col items-center justify-center h-full text-brownish-400 dark:text-brownish-50">
    <BaseIcon :path="mdiFileDocumentOutline" :size="64" />
    <p class="mt-4 text-sm">{{ $t('notes.emptyState') }}</p>
  </div>
</template>
