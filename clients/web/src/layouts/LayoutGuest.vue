<script setup lang="ts">
import { store as style } from '@/stores/style'

const styleStore = style()
</script>

<template>
  <div :class="{ dark: styleStore.darkMode }">
    <div class="bg-gray-50 dark:bg-slate-800 dark:text-slate-100">
      <slot />
    </div>
  </div>
</template>
