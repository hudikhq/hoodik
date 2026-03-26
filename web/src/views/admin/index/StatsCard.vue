<script setup lang="ts">
import { stats } from '!/admin/files'
import { computed, onMounted, ref } from 'vue'
import CardBox from '@/components/ui/CardBox.vue'
import CardBoxComponentHeader from '@/components/ui/CardBoxComponentHeader.vue'
import { formatSize } from '!/index'
import StatsTable from '@/components/files/stats/StatsTable.vue'
import type { Response } from 'types/admin/files'
import { mdiRefresh } from '@mdi/js'

const props = defineProps<{
  class?: string
}>()

const data = ref<Response>()

const usedBytes = computed(() => data.value?.stats.reduce((acc, item) => acc + item.size, 0) ?? 0)
const totalBytes = computed(() => data.value?.available_space ?? 0)

const usedSpace = computed(() => formatSize(usedBytes.value))
const availableSpace = computed(() => totalBytes.value ? formatSize(totalBytes.value) : '—')

const usagePercent = computed(() => {
  if (!totalBytes.value) return 0
  return Math.min(100, Math.round((usedBytes.value / totalBytes.value) * 100))
})

const usageColor = computed(() => {
  if (usagePercent.value >= 90) return 'bg-redish-500'
  if (usagePercent.value >= 70) return 'bg-orangy-400'
  return 'bg-greeny-500'
})

const refresh = async () => {
  data.value = await stats()
}

onMounted(refresh)
</script>
<template>
  <div :class="props.class">
    <CardBox class="flex w-full">
      <CardBoxComponentHeader title="Storage Overview" :button-icon="mdiRefresh" @button-click="refresh" />

      <div class="-mx-4 -mb-4">
        <!-- Usage bar -->
        <div class="px-4 py-3 border-b border-brownish-100 dark:border-brownish-700/50">
          <div class="flex justify-between items-baseline mb-1.5">
            <span class="text-sm font-medium">{{ usedSpace }} used</span>
            <span class="text-xs text-brownish-400">of {{ availableSpace }}</span>
          </div>
          <div class="h-2 bg-brownish-100 dark:bg-brownish-700 rounded-full overflow-hidden">
            <div
              :class="usageColor"
              class="h-2 rounded-full transition-[width] duration-700"
              :style="{ width: usagePercent + '%' }"
            />
          </div>
          <p class="text-xs text-brownish-400 mt-1.5">{{ usagePercent }}% of capacity used</p>
        </div>

        <!-- Breakdown by type -->
        <div class="px-4 py-3" v-if="data">
          <StatsTable :data="data.stats" :max="data.available_space" />
        </div>
      </div>
    </CardBox>
  </div>
</template>
