<script setup lang="ts">
import { computed, normalizeClass, type PropType } from 'vue'

const props = defineProps({
  path: {
    type: String,
    required: true
  },
  w: {
    type: String,
    default: 'w-6'
  },
  h: {
    type: String,
    default: 'h-6'
  },
  size: {
    type: [String, Number],
    default: null,
    required: false
  },
  // Accept the full Vue class-binding surface (string, array, object-of-booleans)
  // so callers can use the same patterns they'd use on native elements.
  class: {
    type: [String, Array, Object] as PropType<
      string | string[] | Record<string, boolean>
    >,
    default: '',
    required: false
  },
  flex: {
    type: Boolean,
    default: true
  }
})

const spanClass = computed(
  () =>
    `${props.flex ? 'inline-flex' : ''} justify-center items-center ${props.w} ${props.h} ${
      normalizeClass(props.class)
    }`
)

const iconSize = computed(() => props.size ?? 16)
</script>

<template>
  <span :class="spanClass">
    <svg viewBox="0 0 24 24" :width="iconSize" :height="iconSize" class="inline-block">
      <path fill="currentColor" :d="path" />
    </svg>
    <slot />
  </span>
</template>
