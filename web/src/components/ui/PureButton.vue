<script setup lang="ts">
import type { RouteLocationRaw } from 'vue-router'
import { computed } from 'vue'
import BaseIcon from './BaseIcon.vue'

const props = defineProps<{
  icon?: string
  iconSize?: number
  label?: string
  /** Hover-tooltip text; falls back to `label` so existing call sites
   *  don't have to change to keep their accessible label. */
  title?: string
  type?: 'submit' | 'reset' | 'button'
  class?: string | { [key: string]: boolean } | string[]
  to?: RouteLocationRaw
}>()

const tooltip = computed(() => props.title ?? props.label)
</script>
<template>
  <router-link v-if="props.to" :to="props.to" :class="props.class || ''" :title="tooltip">
    <BaseIcon v-if="props.icon" class="m-1" :path="icon as string" :size="iconSize || 15" />
    <span v-if="props.label">{{ label }}</span>
    <slot></slot>
  </router-link>
  <button v-else :type="props.type || 'button'" :class="props.class || ''" :title="tooltip">
    <BaseIcon v-if="props.icon" class="m-1" :path="icon as string" :size="iconSize || 15" />
    <span v-if="props.label">{{ label }}</span>
    <slot></slot>
  </button>
</template>
