<script setup lang="ts">
import CardBox from '@/components/ui/CardBox.vue'
import CardBoxComponentHeader from '@/components/ui/CardBoxComponentHeader.vue'
import StatsTable from '@/components/files/stats/StatsTable.vue'
import { store as filesStore } from '!/storage'
import { computedAsync } from '@vueuse/core'
import { computed } from 'vue'
import { formatSize } from '!/index'

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
</script>
<template>
  <CardBox :class="props.class">
    <CardBoxComponentHeader title="Storage Usage">
      <div class="flex items-center px-4 py-3 text-sm text-brownish-400">
        {{ usedSpace }}<span v-if="quota"> / {{ quota }}</span><span v-else> / unlimited</span>
      </div>
    </CardBoxComponentHeader>

    <div v-if="data" class="-mx-4 -mb-4">
      <div v-if="quota" class="px-4 pt-3 pb-2">
        <div class="flex justify-between text-xs text-brownish-400 mb-1.5">
          <span>{{ usagePercent }}% used</span>
          <span>{{ quota }} total</span>
        </div>
        <div class="h-2 rounded-full bg-brownish-100 dark:bg-brownish-700 overflow-hidden">
          <div
            class="h-full rounded-full transition-[width] duration-700"
            :class="usageColor"
            :style="{ width: usagePercent + '%' }"
          />
        </div>
      </div>

      <StatsTable :data="data.stats" :max="data.quota" />
    </div>
  </CardBox>
</template>
