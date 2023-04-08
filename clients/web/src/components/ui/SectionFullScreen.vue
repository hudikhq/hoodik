<script setup lang="ts">
import { computed } from 'vue'
import { store as style } from '@/stores/style'
import { gradientBgPurplePink, gradientBgDark, gradientBgPinkRed } from '@/colors'

const props = defineProps<{ bg: 'purplePink' | 'pinkRed' }>()

const colorClass = computed(() => {
  if (style().darkMode) {
    return gradientBgDark
  }

  switch (props.bg) {
    case 'purplePink':
      return gradientBgPurplePink
    case 'pinkRed':
      return gradientBgPinkRed
  }

  return ''
})
</script>

<template>
  <div class="flex min-h-screen items-center justify-center" :class="colorClass">
    <slot card-class="w-11/12 md:w-7/12 lg:w-6/12 xl:w-4/12 shadow-2xl" />
  </div>
</template>
