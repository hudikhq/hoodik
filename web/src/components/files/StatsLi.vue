<script setup lang="ts">
import { formatSize } from '!/index'
import { store as filesStore } from '!/storage'
import { computed } from 'vue'

const Storage = filesStore()

const used = computed(() => {
  return formatSize(Storage.stats?.used_space || 0)
})

const available = computed(() => {
  if (!Storage.stats?.quota) {
    return 'unlimited'
  }

  return formatSize(Storage.stats.quota)
})
</script>
<template>
  <li v-if="Storage.stats" class="text-center">Used {{ used }} of {{ available }}</li>
</template>
