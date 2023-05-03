<script setup lang="ts">
import { colorsText, colorsBgLight, type ColorType } from '@/colors.js'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { computed } from 'vue'

const props = defineProps<{
  icon: string
  color?: ColorType
  w?: string
  h?: string
  bg?: Boolean
}>()

const componentClass = computed(() => {
  if (props.bg && props.color) {
    return colorsBgLight[props.color]
  }

  const cls = props.color ? colorsText[props.color] : ''

  return [cls, 'bg-brownish-50 dark:bg-brownish-800']
})
</script>

<template>
  <BaseIcon
    :path="icon"
    :w="w || 'w-12'"
    :h="h || 'h-12'"
    size="24"
    class="rounded-full"
    :class="componentClass"
  />
</template>
