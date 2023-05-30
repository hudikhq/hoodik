<script setup lang="ts">
import type { RouteLocationRaw } from 'vue-router'
import BaseIcon from './BaseIcon.vue'

const props = defineProps<{
  icon?: string
  iconSize?: number
  label?: string
  type?: 'submit' | 'reset' | 'button'
  class?: string | { [key: string]: boolean } | string[]
  to?: RouteLocationRaw
}>()
</script>
<template>
  <router-link v-if="props.to" :to="props.to" :class="props.class || ''" :title="props.label">
    <BaseIcon v-if="props.icon" class="m-1" :path="icon as string" :size="iconSize || 15" />
    <span v-if="props.label">{{ label }}</span>
    <slot></slot>
  </router-link>
  <button v-else :type="props.type || 'button'" :class="props.class || ''" :title="props.label">
    <BaseIcon v-if="props.icon" class="m-1" :path="icon as string" :size="iconSize || 15" />
    <span v-if="props.label">{{ label }}</span>
    <slot></slot>
  </button>
</template>
