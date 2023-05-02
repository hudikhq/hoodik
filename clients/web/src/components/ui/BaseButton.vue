<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink } from 'vue-router'
import { getButtonColor, type ColorType } from '@/colors'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import type { RouteLocation } from 'vue-router'

const props = defineProps<{
  label?: string | number
  icon?: string
  iconSize?: number
  href?: string
  target?: string
  to?: string | RouteLocation
  type?: string
  color?: ColorType
  as?: string
  xs?: Boolean
  small?: Boolean
  outline?: Boolean
  active?: Boolean
  disabled?: Boolean
  roundedFull?: Boolean
  noBorder?: Boolean
  class?: String
  dropdownEl?: boolean
}>()

const is = computed(() => {
  if (props.as) {
    return props.as
  }

  if (props.to) {
    return RouterLink
  }

  if (props.href) {
    return 'a'
  }

  return 'button'
})

const computedType = computed(() => {
  if (is.value === 'button') {
    return props.type ?? 'button'
  }

  return null
})

const labelClass = computed(() => {
  if (props.xs) {
    return 'px-1'
  }

  if (props.small && props.icon) {
    return 'px-1'
  }

  return 'px-2'
})

const componentClass = computed(() => {
  let base = [
    props.dropdownEl ? '' : 'inline-flex',
    props.dropdownEl ? 'justify-start' : 'justify-center',
    'items-center',
    'whitespace-nowrap',
    'focus:outline-none',
    'transition-colors',
    'focus:ring',
    'duration-150',
    props.disabled ? 'cursor-not-allowed' : 'cursor-pointer',
    props.roundedFull ? 'rounded-full' : 'rounded',
    getButtonColor(props.color || 'white', !!props.outline, !props.disabled, !!props.active)
  ]

  if (!props.noBorder) {
    base.push('border')
  }

  if (!props.label && props.icon) {
    base.push('p-1')
  } else if (props.xs) {
    base.push('text-xs')
    base.push('py-1', props.roundedFull ? 'px-3' : 'px-1')
  } else if (props.small) {
    base.push('text-sm', props.roundedFull ? 'px-3 py-1' : 'p-1')
  } else {
    base.push('py-2', props.roundedFull ? 'px-6' : 'px-3')
  }

  if (props.disabled) {
    base.push(props.outline ? 'opacity-50' : 'opacity-70')
  }

  if (props.class) {
    base.push(props.class as string)
  }

  return base
})
</script>

<template>
  <component
    :is="is"
    :class="componentClass"
    :href="href"
    :type="computedType"
    :to="to"
    :target="target"
    :disabled="disabled"
  >
    <BaseIcon v-if="icon" :path="icon" :size="iconSize" />
    <span v-if="label" :class="labelClass">{{ label }}</span>
  </component>
</template>
