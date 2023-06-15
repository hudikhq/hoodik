<script setup lang="ts">
import type { Stats } from 'types/admin/files'
import StatsRow from './StatsRow.vue'

const props = defineProps<{
  data: Stats[]
  max: number
}>()
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
