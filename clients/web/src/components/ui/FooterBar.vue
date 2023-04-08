<script setup lang="ts">
import { containerMaxW } from '@/config.js'
import BaseLevel from '@/components/ui/BaseLevel.vue'

const year = new Date().getFullYear()
</script>

<template>
  <footer class="py-2 px-6" :class="containerMaxW">
    <BaseLevel>
      <div class="text-center md:text-left">
        <b>{{ year }} Hoodik E2EE Drive</b>
        <slot />
      </div>
    </BaseLevel>
  </footer>
</template>
