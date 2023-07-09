<script setup lang="ts">
import { RouterView } from 'vue-router'
</script>

<template>
  <notifications class="m-4 text-lg" :pause-on-hover="true" />
  <RouterView />
</template>
