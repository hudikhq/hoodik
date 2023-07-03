<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  text: string
  class?: string | { [key: string]: boolean }
  middle?: boolean
}>()

const leftHalf = computed(() => {
  if (!props.middle) {
    return props.text
  }

  let t = props.text
  if (props.text.length > 9) {
    return t.substring(0, Math.floor(t.length / 2))
  }

  return t
})

const rightHalf = computed(() => {
  if (!props.middle) {
    return ''
  }

  let t = props.text
  if (t.length > 9) {
    return t.substring(Math.floor(t.length / 2), t.length)
  }

  return ''
})

const leftStyle = computed(() => {
  if (!props.middle) {
    return 'width: 100%;'
  }

  if (props.text.length > 9) {
    return 'width: 50%;'
  }

  return ''
})
</script>
<template>
  <span :class="[props.class, 'upper-container']">
    <span class="overflow-left" :style="leftStyle">{{ leftHalf }}</span>
    <span class="overflow-outer-right">
      <span class="overflow-inner-right">{{ rightHalf }}</span>
    </span>
  </span>
</template>
<style lang="css">
.upper-container {
  width: 100%;
  display: inline-block;
  display: flex;
  flex-wrap: wrap;
  white-space: nowrap;
}
.overflow-left {
  overflow: hidden;
  text-overflow: ellipsis;
  flex-grow: 1;
}
.overflow-outer-right {
  width: 50%;
  text-align: right;
  overflow: hidden;
  white-space: nowrap;
}
.overflow-inner-right {
  float: right;
}
</style>
