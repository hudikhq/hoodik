<script setup lang="ts">
import { containerMaxW } from '@/config'
</script>

<template>
  <section class="pt-6 xl:p-6" :class="containerMaxW">
    <slot />
  </section>
</template>
