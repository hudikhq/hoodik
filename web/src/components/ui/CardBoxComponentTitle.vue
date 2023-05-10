<script setup lang="ts">
import BaseIcon from './BaseIcon.vue'
defineProps({
  icon: {
    type: String,
    required: false
  },
  title: {
    type: String,
    required: false
  }
})
</script>

<template>
  <div class="flex items-center justify-between mb-3">
    <div class="flex justify-start items-left space-x-2">
      <BaseIcon v-if="icon" :path="icon" :size="30" :flex="false" class="mr-2" />
      <h1 class="text-2xl" v-if="title">
        {{ title }}
      </h1>
    </div>
    <slot />
  </div>
</template>
