<script setup lang="ts">
import { defineAsyncComponent } from 'vue'
import type { Preview } from '!/preview'
import NotesEmptyState from './NotesEmptyState.vue'

const PreviewMarkdown = defineAsyncComponent(
  () => import('@/components/preview/PreviewMarkdown.vue')
)

defineProps<{
  preview?: Preview
}>()
</script>

<template>
  <div class="flex-1 min-w-0 h-full overflow-hidden">
    <Suspense v-if="preview">
      <PreviewMarkdown :modelValue="preview" />
    </Suspense>
    <NotesEmptyState v-else />
  </div>
</template>
