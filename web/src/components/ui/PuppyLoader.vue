<script setup lang="ts">
import { computed } from 'vue'
import SpinnerIcon from '@/components/ui/SpinnerIcon.vue'

const props = defineProps<{
  modelValue?: boolean
  opacity?: number
}>()

const opacity = computed(() => {
  return props.opacity || 1
})
</script>
<template>
  <div
    class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-10
      flex flex-col items-center gap-2 px-5 py-4 rounded-xl
      bg-white/85 dark:bg-brownish-800/85 backdrop-blur-sm
      border border-brownish-200/40 dark:border-brownish-600/60
      text-brownish-400 dark:text-brownish-100 shadow-sm"
    :style="{ opacity }"
    role="status"
    aria-live="polite"
    v-if="props.modelValue || typeof props.modelValue === 'undefined'"
  >
    <SpinnerIcon :size="24" />

    <slot>
      <span class="text-sm">Working…</span>
    </slot>
  </div>
</template>
