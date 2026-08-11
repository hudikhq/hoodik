<script setup lang="ts">
import BaseIcon from '@/components/ui/BaseIcon.vue'

defineProps({
  title: {
    type: String,
    required: true
  },
  icon: {
    type: String,
    default: null
  },
  buttonIcon: {
    type: String,
    default: null
  },
  /** Names the icon-only header button; it has no text of its own. */
  buttonLabel: {
    type: String,
    default: null
  }
})

const emit = defineEmits(['button-click'])

const buttonClick = (event: Event) => {
  emit('button-click', event)
}
</script>

<template>
  <header
    class="flex flex-wrap items-stretch border-b border-paper-200 dark:border-brownish-800"
  >
    <div class="flex items-center py-3 grow font-bold" :class="[icon ? 'px-4' : 'px-6']">
      <BaseIcon v-if="icon" :path="icon" class="mr-3" />
      <h1 class="text-2xl">{{ title }}</h1>
      <button
        v-if="buttonIcon"
        :title="buttonLabel ?? title"
        :aria-label="buttonLabel ?? title"
        class="flex items-center ml-2 justify-center ring-redish-700 focus:ring"
        @click="buttonClick"
      >
        <BaseIcon :path="buttonIcon" />
      </button>
    </div>
    <slot />
  </header>
</template>
