<script setup lang="ts">
import { store } from '@/stores/style'

defineProps({
  display: {
    type: String,
    default: 'flex'
  },
  useMargin: Boolean
})

const styleStore = store()
</script>

<template>
  <div
    :class="[
      display,
      styleStore.navBarItemLabelStyle,
      styleStore.navBarItemLabelHoverStyle,
      useMargin ? 'my-2 mx-3' : 'py-2 px-3'
    ]"
    class="items-center cursor-pointer dark:text-white dark:hover:text-brownish-400"
  >
    <slot />
  </div>
</template>
