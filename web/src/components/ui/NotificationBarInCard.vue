<script setup lang="ts">
import { colorsBgLight, type ColorType } from '@/colors.js'

defineProps<{
  color: ColorType
}>()
</script>

<template>
  <div class="flex flex-col mb-6 -mt-6 -mr-6 -ml-6 animate-fade-in">
    <div
      :class="[colorsBgLight[color] || '']"
      class="rounded-t-2xl flex flex-col p-6 transition-colors"
    >
      <slot />
    </div>
  </div>
</template>
