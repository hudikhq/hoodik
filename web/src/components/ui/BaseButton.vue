<script setup lang="ts">
import { computed, useAttrs } from 'vue'
import { getButtonColor, type ColorType } from '@/colors'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import type { RouteLocation } from 'vue-router'

const props = defineProps<{
  label?: string | number
  icon?: string
  iconSize?: number
  href?: string
  target?: string
  to?: RouteLocation | { name: string }
  type?: string
  color?: ColorType
  as?: string
  // Primitive `boolean`, not the `Boolean` wrapper: Vue only applies
  // valueless-attribute casting for the primitive, so `<BaseButton small />`
  // used to arrive as the empty string and every flag silently did nothing.
  xs?: boolean
  small?: boolean
  outline?: boolean
  active?: boolean
  disabled?: boolean
  roundedFull?: boolean
  notRounded?: boolean
  noBorder?: boolean
  class?: String
  dropdownEl?: boolean
}>()

const is = computed(() => {
  if (props.as) {
    return props.as
  }

  if (props.to) {
    return 'router-link'
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

// Dense controls — toolbar icons and the xs/small steps — keep the tighter
// 4px corner; anything standard-sized takes the 8px button radius.
const isDense = computed(() => !!props.icon || !!props.xs || !!props.small)

const attrs = useAttrs()

/**
 * An icon with no visible label has no accessible name of its own. `title`
 * alone technically supplies one, but it never reaches touch users and only
 * reaches keyboard users on hover, so the tooltip text is promoted to a real
 * label. An explicit `aria-label` from the caller always wins.
 */
const iconLabel = computed(() => {
  if (props.label || !props.icon) return undefined
  return (attrs['aria-label'] as string) ?? (attrs.title as string) ?? undefined
})

const componentClass = computed(() => {
  let base = [
    props.dropdownEl ? '' : 'inline-flex',
    props.dropdownEl ? 'justify-start' : 'justify-center',
    'items-center',
    'whitespace-nowrap',
    'focus:outline-none',
    'transition-colors',
    'focus-visible:ring',
    'duration-150',
    props.disabled ? 'cursor-not-allowed' : 'cursor-pointer',
    props.roundedFull ? 'rounded-full' : props.notRounded ? '' : isDense.value ? 'rounded' : 'rounded-lg',
    getButtonColor(props.color || 'light', !!props.outline, !props.disabled, !!props.active)
  ]

  if (!props.noBorder) {
    base.push('border')
  }

  if (props.icon) {
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
    v-if="is !== 'router-link'"
    :is="is"
    :class="componentClass"
    :href="href"
    :type="computedType"
    :to="to"
    :target="target"
    :disabled="disabled"
    :aria-label="iconLabel"
  >
    <BaseIcon v-if="icon" :path="icon" :size="iconSize" />
    <span v-if="label" :class="labelClass">{{ label }}</span>
  </component>
  <router-link
    v-else-if="to"
    :class="componentClass"
    :type="computedType"
    :to="to"
    :target="target"
    :disabled="disabled"
    :aria-label="iconLabel"
  >
    <BaseIcon v-if="icon" :path="icon" :size="iconSize" />
    <span v-if="label" :class="labelClass">{{ label }}</span>
  </router-link>
</template>
