<script setup lang="ts">
import type { FormType } from '.'
import { computed, ref } from 'vue'
import * as colors from '@/colors'
import type { ColorType } from '@/colors'

const originalClass =
  'inline-flex justify-center items-center whitespace-nowrap focus:outline-none transition-colors focus:ring duration-150 border cursor-pointer rounded py-2 px-3 mr-3 last:mr-0 mb-3'

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
}>()

const computedClass = computed(() => {
  let combined = `${originalClass}`
  combined = `${combined} ${colors
    .getButtonColor(
      props.color || 'success',
      props.outline || true,
      props.hasHover || true,
      !!props.isActive
    )
    .join('')}`

  return combined
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

const componentClass = ref<string>(
  `${props.class ? props.class : (props.classAdd || '') + ' ' + computedClass.value}`
)
</script>
<template>
  <button
    :type="type || 'submit'"
    :disabled="isDisabled || false"
    @click="reset"
    :class="{
      [componentClass]: true,
      'border border-green-300': isDisabled
    }"
  >
    <slot></slot>
  </button>
</template>
