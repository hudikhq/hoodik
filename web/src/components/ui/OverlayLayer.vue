<script setup lang="ts">
import { store as style } from '!/style.js'
import { watch } from 'vue'
const styleStore = style()

const props = defineProps({
  visible: Boolean,
  zIndex: {
    type: String,
    default: 'z-50'
  },
  type: {
    type: String,
    default: 'flex'
  }
})

watch(
  () => props.visible,
  (value) => {
    if (value) {
      document.body.style.overflow = 'hidden'
    } else {
      document.body.style.overflow = 'auto'
    }
  }
)

const emit = defineEmits(['overlay-click'])

const overlayClick = (event: Event) => {
  emit('overlay-click', event)
}
</script>

<template>
  <div
    v-show="visible"
    :class="[type, zIndex]"
    class="items-center flex-col justify-center overflow-hidden fixed inset-0"
  >
    <transition
      enter-active-class="transition duration-150 ease-in"
      enter-from-class="opacity-0"
      enter-to-class="opacity-100"
      leave-active-class="transition duration-150 ease-in"
      leave-from-class="opacity-100"
      leave-to-class="opacity-0"
    >
      <div
        class="absolute inset-0 bg-gradient-to-tr opacity-90 dark:from-brownish-700 dark:via-brownish-900 dark:to-brownish-700"
        :class="styleStore.overlayStyle"
        @click="overlayClick"
      />
    </transition>
    <transition
      enter-active-class="transition duration-100 ease-out"
      enter-from-class="transform scale-95 opacity-0"
      enter-to-class="transform scale-100 opacity-100"
      leave-active-class="animate-fade-out"
    >
      <slot />
    </transition>
  </div>
</template>
