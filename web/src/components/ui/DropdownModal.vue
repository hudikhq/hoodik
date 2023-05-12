<script setup lang="ts">
import { computed } from 'vue'
import CardBox from '@/components/ui/CardBox.vue'
import OverlayLayer from '@/components/ui/OverlayLayer.vue'
import type { FormType } from '../form'

const props = defineProps<{
  title?: string
  modelValue: boolean | undefined
  form?: FormType
}>()

const emit = defineEmits(['update:modelValue', 'cancel', 'confirm'])

const value = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value)
})

const cancel = () => {
  if (props.form) {
    props.form.handleReset()
  }

  value.value = false
  emit('cancel')
}

window.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && value.value) {
    cancel()
  }
})
</script>

<template>
  <OverlayLayer :visible="value" @overlay-click="cancel">
    <CardBox
      v-show="value"
      class="shadow-lg max-h-modal w-8/12 md:w-3/5 lg:w-2/5 xl:w-1/12 z-50 py-2"
      rounded="rounded-md"
      :hasComponentLayout="true"
      is-modal
    >
      <slot />
    </CardBox>
  </OverlayLayer>
</template>
