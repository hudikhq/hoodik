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
</script>
<template>
  <!-- Sits in the aside rail, which is charcoal in both themes — the light
       step here would be measured against the wrong ground. -->
  <li v-if="Storage.stats" class="text-center text-sm text-brownish-50">
    {{ $t('files.stats.usedOf', { used, available }) }}
  </li>
</template>
