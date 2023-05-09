<script setup lang="ts">
import type { FormType } from '.'
import { computed } from 'vue'
import { getButtonColor, type ColorType } from '@/colors'
import BaseIcon from '../ui/BaseIcon.vue'

const props = defineProps<{
  disabled?: boolean
  type?: 'submit' | 'button' | 'reset'
  form?: FormType
  class?: string
  classAdd?: string
  color?: ColorType
  outline?: boolean
  hasHover?: boolean
  isActive?: boolean
  roundedFull?: boolean
  dropdownEl?: boolean
  noBorder?: boolean
  active?: boolean
  label?: string
  icon?: string
  iconSize?: number
  xs?: boolean
  small?: boolean
}>()

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
    props.classAdd || '',
    props.disabled ? 'cursor-not-allowed' : 'cursor-pointer',
    props.roundedFull ? 'rounded-full' : 'rounded',
    getButtonColor(props.color || 'light', !!props.outline, !props.disabled, !!props.active)
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

const labelClass = computed(() => {
  if (props.xs) {
    return 'px-1'
  }

  if (props.small && props.icon) {
    return 'px-1'
  }

  return 'px-2'
})

const emit = defineEmits(['click'])

const reset = (e: Event) => {
  if (props.type === 'reset' && props.form) {
    e.preventDefault()
    props.form.handleReset()
  }

  if (props.type === 'button') {
    e.preventDefault()
    emit('click', e)
  }
}

const isDisabled = computed(() => {
  return props.disabled || props.form?.isSubmitting.value
})
</script>
<template>
  <button
    :type="type || 'submit'"
    :disabled="isDisabled || false"
    @click="reset"
    :class="componentClass"
  >
    <BaseIcon v-if="icon" :path="icon" :size="iconSize" />
    <span v-if="label" :class="labelClass">{{ label }}</span>
    <slot></slot>
  </button>
</template>
