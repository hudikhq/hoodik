<script setup lang="ts">
import { RouterView, useRouter } from 'vue-router'
import DebugOverlay from '@/components/ui/DebugOverlay.vue'
import RouteLoadingBar from '@/components/ui/RouteLoadingBar.vue'

// The boot splash in index.html sits outside #app, because mounting only
// means the shell arrived — the first view is a lazy chunk that may still
// be downloading. Router readiness is the moment something real renders.
useRouter()
  .isReady()
  .finally(() => document.getElementById('boot')?.remove())
</script>

<template>
  <notifications class="m-4 text-lg" :pause-on-hover="true" />
  <RouteLoadingBar />
  <RouterView />
  <DebugOverlay />
</template>
