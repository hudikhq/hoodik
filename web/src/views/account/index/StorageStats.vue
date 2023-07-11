<script setup lang="ts">
import CardBox from '@/components/ui/CardBox.vue'
import CardBoxComponentHeader from '@/components/ui/CardBoxComponentHeader.vue'
import StatsTable from '@/components/files/stats/StatsTable.vue'
import { store as filesStore } from '!/storage'
import { computedAsync } from '@vueuse/core'
import { computed } from 'vue'
import { formatSize } from '!/index'

const storage = filesStore()
const data = computedAsync(async () => {
  await storage.loadStats()
  return storage.stats
})

const quota = computed(() => {
  if (!data.value?.quota) return 'unlimited'

  return formatSize(data.value.quota)
})

const usedSpace = computed(() => {
  if (!data.value?.used_space) return '0.00B'

  return formatSize(data.value?.used_space)
})
</script>
<template>
  <CardBox class="sm:w-1/2">
    <CardBoxComponentHeader title="Storage usage" class="mb-4">
      <div class="mt-4" title="Storage capacity usage">{{ usedSpace }} / {{ quota }}</div>
    </CardBoxComponentHeader>

    <StatsTable v-if="data" :data="data.stats" :max="data.quota" />
  </CardBox>
</template>
