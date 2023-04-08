<script setup lang="ts">
import { computed } from 'vue'
import { colorsBgLight, colorsOutline, type ColorType } from '@/colors'
import PillTagPlain from '@/components/ui/PillTagPlain.vue'

const props = defineProps<{
  label: string
  color: ColorType
  icon?: string
  small?: Boolean
  outline?: Boolean
}>()

const componentClass = computed(() => [
  props.small ? 'py-1 px-3' : 'py-1.5 px-4',
  // @ts-ignore
  props.outline ? colorsOutline[props.color] : colorsBgLight[props.color]
])
</script>

<template>
  <PillTagPlain
    class="border rounded-full"
    :class="componentClass"
    :icon="icon"
    :label="label"
    :small="!!small"
  />
</template>
