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
  isModal: Boolean
})

const emit = defineEmits(['submit'])

const slots = useSlots()

const hasFooterSlot = computed(() => slots.footer && !!slots.footer())

const componentClass = computed(() => {
  // A modal has to read as a surface sitting above the page, and in dark mode
  // the page, the scrim and the card were all within a shade of each other —
  // nothing separated the dialog from what it covered. Modals get the lighter
  // surface, a border that survives against the scrim, and a real shadow;
  // inline cards keep the flatter treatment that suits them in a page flow.
  // Inline cards carry their edge on the seam alone — a resting shadow would
  // float them off a page that is meant to read as one flat surface.
  return [
    props.rounded,
    props.flex,
    'border',
    props.isModal
      ? 'border-paper-300 dark:border-brownish-600 shadow-2xl'
      : 'border-paper-300 dark:border-brownish-700/40',
    props.isModal ? 'dark:bg-brownish-800' : 'dark:bg-brownish-900'
  ]
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
