<script setup lang="ts">
import CardBox from '@/components/ui/CardBox.vue'
import StatsTable from '@/components/files/stats/StatsTable.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { store as filesStore } from '!/storage'
import { computedAsync } from '@vueuse/core'
import { computed } from 'vue'
import { formatSize } from '!/index'
import { mdiChartDonut, mdiDatabase } from '@mdi/js'

const props = defineProps<{
  class?: string
}>()

const storage = filesStore()
const data = computedAsync(async () => {
  await storage.loadStats()
  return storage.stats
})

const quota = computed(() => {
  if (!data.value?.quota) return null
  return formatSize(data.value.quota)
})

const usedSpace = computed(() => {
  if (!data.value?.used_space) return '0 B'
  return formatSize(data.value?.used_space)
})

const usagePercent = computed(() => {
  if (!data.value?.quota || !data.value?.used_space) return 0
  return Math.min(100, Math.round((data.value.used_space / data.value.quota) * 100))
})

const usageColor = computed(() => {
  if (usagePercent.value >= 90) return 'bg-redish-500'
  if (usagePercent.value >= 70) return 'bg-orangy-400'
  return 'bg-greeny-500'
})

const usageTextColor = computed(() => {
  if (usagePercent.value >= 90) return 'text-redish-500'
  if (usagePercent.value >= 70) return 'text-orangy-400'
  return 'text-greeny-500'
})
</script>
<template>
  <CardBox :class="props.class">
    <div class="-mx-4 -mt-4 px-6 py-6 border-b border-brownish-100 dark:border-brownish-700/50 rounded-t-2xl">
      <div class="flex items-center gap-2 mb-4">
        <BaseIcon :path="mdiDatabase" :size="14" class="text-brownish-400 dark:text-brownish-500" />
        <p class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-500">Storage Usage</p>
      </div>

      <div class="flex items-end justify-between mb-3">
        <div>
          <span class="text-2xl font-bold" :class="usageTextColor">{{ usedSpace }}</span>
          <span class="text-sm text-brownish-400 ml-1">
            <span v-if="quota">/ {{ quota }}</span>
            <span v-else>/ unlimited</span>
          </span>
        </div>
        <span v-if="quota" class="text-sm font-semibold" :class="usageTextColor">{{ usagePercent }}%</span>
      </div>

      <div v-if="quota" class="h-3 rounded-full bg-brownish-100 dark:bg-brownish-700 overflow-hidden">
        <div
          class="h-full rounded-full transition-[width] duration-700"
          :class="usageColor"
          :style="{ width: usagePercent + '%' }"
        />
      </div>
    </div>

    <div v-if="data" class="-mx-4 -mb-4">
      <div class="px-6 pt-4 pb-1">
        <div class="flex items-center gap-2 mb-2">
          <BaseIcon :path="mdiChartDonut" :size="14" class="text-brownish-400 dark:text-brownish-500" />
          <p class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-500">By File Type</p>
        </div>
      </div>
      <div class="px-4 pb-4">
        <StatsTable :data="data.stats" :max="data.quota" />
      </div>
    </div>
  </CardBox>
</template>
