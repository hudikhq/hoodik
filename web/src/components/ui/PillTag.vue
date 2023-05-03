<script setup lang="ts">
import { computed } from 'vue'
import { colorsBgLight, colorsOutline, type ColorType } from '@/colors'
import PillTagPlain from '@/components/ui/PillTagPlain.vue'

const props = defineProps<{
  label: string
  color?: ColorType
  icon?: string
  small?: boolean
  outline?: boolean
  class?: string
  notRound?: boolean
  noBorder?: boolean
}>()

const componentClass = computed(() => [
  props.small ? 'py-1 px-3' : 'py-1.5 px-4',
  props.outline ? colorsOutline[props.color || 'empty'] : colorsBgLight[props.color || 'empty'],
  props.class ? props.class : '',
  props.noBorder ? '' : 'border',
  props.notRound ? '' : 'rounded-full'
])
</script>

<template>
  <PillTagPlain :class="componentClass" :icon="icon" :label="label" :small="!!small" />
</template>
