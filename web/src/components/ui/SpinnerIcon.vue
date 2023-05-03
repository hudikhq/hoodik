<script setup lang="ts">
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { mdiLoading } from '@mdi/js'

const props = defineProps({
  w: {
    type: String,
    default: 'w-6'
  },
  h: {
    type: String,
    default: 'h-6'
  },
  size: {
    type: [String, Number],
    default: null,
    required: false
  },
  path: {
    type: String,
    default: mdiLoading,
    required: false
  }
})
</script>

<template>
  <BaseIcon :path="props.path" class="spinner" :size="props.size" />
</template>

<style scoped lang="css">
.spinner {
  animation: rotate 1s linear infinite;
}
@keyframes rotate {
  to {
    transform: rotate(359deg); /* some browsers don't display spin when it is 360 deg */
  }
}
</style>
