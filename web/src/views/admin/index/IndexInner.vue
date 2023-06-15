<script setup lang="ts">
import { stats } from '!/admin/files'
import { computed, onMounted, ref } from 'vue'
import SectionMain from '@/components/ui/SectionMain.vue'
import CardBox from '@/components/ui/CardBox.vue'
import CardBoxComponentHeader from '@/components/ui/CardBoxComponentHeader.vue'
import CardBoxComponentFooter from '@/components/ui/CardBoxComponentFooter.vue'
import { formatSize } from '!/index'
import StatsTable from '@/components/files/stats/StatsTable.vue'
import type { Response } from 'types/admin/files'
import { mdiRefresh } from '@mdi/js'

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
  <SectionMain>
    <CardBox class="sm:w-1/2">
      <CardBoxComponentHeader
        title="Files"
        :button-icon="mdiRefresh"
        @button-click="stats"
        class="mb-4"
      />

      <StatsTable v-if="data" :data="data.stats" :max="data.available_space" />
    </CardBox>

    <CardBoxComponentFooter
      >Storage space usage: {{ usedSpace }} / {{ availableSpace }}
    </CardBoxComponentFooter>
  </SectionMain>
</template>
