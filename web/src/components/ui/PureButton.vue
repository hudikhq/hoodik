<script setup lang="ts">
import BaseIcon from './BaseIcon.vue'

const props = defineProps<{
  icon?: string
  iconSize?: number
  label?: string
  type?: 'submit' | 'reset' | 'button'
  class?: string | { [key: string]: boolean } | string[]
}>()
</script>
<template>
  <button :type="props.type || 'button'" :class="props.class || ''">
    <BaseIcon v-if="props.icon" class="m-1" :path="icon as string" :size="iconSize || 15" />
    <span v-if="props.label">{{ label }}</span>
    <slot></slot>
  </button>
</template>
