<script setup lang="ts">
import { computed, useSlots } from 'vue'
import CardBoxComponentBody from '@/components/ui/CardBoxComponentBody.vue'
import CardBoxComponentFooter from '@/components/ui/CardBoxComponentFooter.vue'

const props = defineProps({
  rounded: {
    type: String,
    default: 'rounded-2xl'
  },
  flex: {
    type: String,
    default: 'flex-col'
  },
  hasComponentLayout: Boolean,
  hasTable: Boolean,
  isForm: Boolean,
  isHoverable: Boolean,
  isModal: Boolean
})

const emit = defineEmits(['submit'])

const slots = useSlots()

const hasFooterSlot = computed(() => slots.footer && !!slots.footer())

const componentClass = computed(() => {
  const base = [
    props.rounded,
    props.flex,
    'border',
    'border-brownish-200/40 dark:border-brownish-700/40',
    'shadow-sm dark:shadow-none',
    props.isModal ? 'dark:bg-brownish-900' : 'dark:bg-brownish-900'
  ]

  if (props.isHoverable) {
    base.push('hover:shadow-lg transition-shadow duration-500')
  }

  return base
})

const submit = (event: Event) => {
  emit('submit', event)
}
</script>

<template>
  <component
    :is="isForm ? 'form' : 'div'"
    :class="componentClass"
    class="bg-white flex overflow-hidden"
    @submit="submit"
  >
    <slot v-if="hasComponentLayout" />
    <template v-else>
      <CardBoxComponentBody :no-padding="hasTable">
        <slot />
      </CardBoxComponentBody>
      <CardBoxComponentFooter v-if="hasFooterSlot">
        <slot name="footer" />
      </CardBoxComponentFooter>
    </template>
  </component>
</template>
