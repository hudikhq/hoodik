<script setup lang="ts">
defineProps({
  noPadding: Boolean
})
</script>

<template>
  <div class="flex-1" :class="{ 'p-4': !noPadding }">
    <slot />
  </div>
</template>
