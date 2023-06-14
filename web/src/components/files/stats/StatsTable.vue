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
    <StatsRow v-for="item in props.data" :key="item.mime" :data="item" :max="max" />
  </div>
</template>
