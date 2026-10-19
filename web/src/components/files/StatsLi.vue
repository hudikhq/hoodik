<script setup lang="ts">
import { formatSize } from '!/index'
import { store as filesStore } from '!/storage'
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const Storage = filesStore()

const used = computed(() => {
  return formatSize(Storage.stats?.used_space || 0)
})

const available = computed(() => {
  if (!Storage.stats?.quota) {
    return t('files.stats.unlimited')
  }

  return formatSize(Storage.stats.quota)
})

const usagePercent = computed(() => {
  if (!Storage.stats?.quota || !Storage.stats?.used_space) return 0
  return Math.min(100, (Storage.stats.used_space / Storage.stats.quota) * 100)
})

const usageColor = computed(() => {
  if (usagePercent.value >= 90) return 'bg-redish-500'
  if (usagePercent.value >= 70) return 'bg-orangy-400'
  return 'bg-greeny-500'
})
</script>
<template>
  <!-- Sits in the aside rail, which is charcoal in both themes — the light
       step here would be measured against the wrong ground. -->
  <li v-if="Storage.stats" class="pb-3 text-center text-sm text-brownish-50">
    {{ $t('files.stats.usedOf', { used, available }) }}
    <div
      v-if="Storage.stats.quota"
      class="mx-6 mt-1.5 h-1.5 rounded-full bg-brownish-700 overflow-hidden"
      role="progressbar"
      :aria-valuenow="Math.round(usagePercent)"
      aria-valuemin="0"
      aria-valuemax="100"
    >
      <!-- min-width keeps any non-zero usage visible against a large quota -->
      <div
        class="h-full rounded-full transition-[width] duration-700"
        :class="[usageColor, Storage.stats.used_space ? 'min-w-1.5' : '']"
        :style="{ width: usagePercent + '%' }"
      />
    </div>
  </li>
</template>
