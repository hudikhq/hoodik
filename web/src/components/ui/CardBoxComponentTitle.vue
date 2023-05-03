<script setup lang="ts">
defineProps({
  title: {
    type: String,
    required: false
  }
})
</script>

<template>
  <div class="flex items-center justify-between mb-3">
    <h1 class="text-2xl" v-if="title">
      {{ title }}
    </h1>
    <slot />
  </div>
</template>
