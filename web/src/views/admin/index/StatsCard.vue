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

const availableSpace = computed(() => {
  if (!data.value) return '0.00B'

  return formatSize(data.value.available_space)
})

const usedSpace = computed(() => {
  if (!data.value) return '0.00B'

  return formatSize(data.value.stats.reduce((acc, item) => acc + item.size, 0))
})

onMounted(async () => {
  data.value = await stats()
})
</script>
<template>
  <div :class="props.class">
    <CardBox class="flex w-full">
      <CardBoxComponentHeader
        title="Storage overview"
        :button-icon="mdiRefresh"
        @button-click="stats"
        class="mb-4"
      >
        <div class="mt-4" title="Storage capacity usage">
          {{ usedSpace }} / {{ availableSpace }}
        </div>
      </CardBoxComponentHeader>

      <StatsTable v-if="data" :data="data.stats" :max="data.available_space" />
    </CardBox>
  </div>
</template>
