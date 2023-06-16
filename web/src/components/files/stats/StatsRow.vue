<script setup lang="ts">
import type { Stats } from 'types/admin/files'
import { computed } from 'vue'
import { formatSize } from '!/index'

const props = defineProps<{
  data: Stats
  max?: number
}>()

const mime = computed(() => props.data.mime)
const count = computed(() => props.data.count)
const size = computed(() => formatSize(props.data.size))
const maxSize = computed(() => {
  if (!props.max) return 'infinity'

  return formatSize(props.max)
})
const percentage = computed(() => {
  if (!props.data.size) return `0%`
  if (!props.max) return `0%`

  return `${Math.round((props.data.size / props.max) * 100)}%`
})
</script>
<template>
  <div class="flex flex-row" :title="`${percentage} of ${maxSize}`">
    <div class="w-6/12 overflow-clip">{{ mime }}</div>
    <div class="w-2/12">{{ count }}</div>
    <div class="w-4/12 text-right">{{ size }}</div>
  </div>
  <div class="bg-greeny-500 h-6 mt-[-23px] opacity-20" :style="{ width: percentage }"></div>
</template>
