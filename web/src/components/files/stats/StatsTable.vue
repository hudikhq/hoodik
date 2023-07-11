<script setup lang="ts">
import type { Stats } from 'types/admin/files'
import StatsRow from './StatsRow.vue'
import SortableName from '@/components/ui/SortableName.vue'
import { computed, ref } from 'vue'

const props = defineProps<{
  data: Stats[]
  max?: number
}>()

const sort = ref({ sort: 'size', order: 'desc' })

const sortedData = computed(() => {
  if (!props.data) return []

  if (!sort.value.sort) {
    return props.data
  }

  const data = props.data

  data.sort((a, b) => {
    if (sort.value.sort === 'mime') {
      return a.mime.localeCompare(b.mime)
    } else if (sort.value.sort === 'count') {
      return a.count - b.count
    } else if (sort.value.sort === 'size') {
      return a.size - b.size
    }

    return 0
  })

  if (sort.value.order === 'desc') {
    return data.reverse()
  } else {
    return data
  }
})
</script>
<template>
  <div class="flex flex-col">
    <div class="flex flex-row border-b-2 border-brownish-500 mb-2">
      <div class="w-6/12">
        <SortableName v-model="sort" name="mime" label="Mime" />
      </div>
      <div class="w-2/12">
        <SortableName v-model="sort" name="count" label="Count" />
      </div>
      <div class="w-4/12">
        <SortableName class="float-right" v-model="sort" name="size" label="Size" />
      </div>
    </div>

    <StatsRow v-for="item in sortedData" :key="item.mime" :data="item" :max="max" />
  </div>
</template>
