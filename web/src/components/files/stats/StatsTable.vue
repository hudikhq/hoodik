<script setup lang="ts">
import type { Stats } from 'types/admin/files'
import StatsRow from './StatsRow.vue'
import { computed } from 'vue'

const props = defineProps<{
  data: Stats[]
}>()

const max = computed(() => {
  return props.data.reduce((acc, item) => {
    if (item.size > acc) {
      return item.size
    }
    return acc
  }, 0)
})
</script>
<template>
  <div class="flex flex-col">
    <div class="flex flex-row border-b-2 border-brownish-500 mb-2">
      <div class="w-6/12">Mime</div>
      <div class="w-2/12">Count</div>
      <div class="w-4/12 text-right">Size</div>
    </div>

    <StatsRow v-for="item in props.data" :key="item.mime" :data="item" :max="max" />
  </div>
</template>
