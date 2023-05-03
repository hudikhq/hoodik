<script setup lang="ts">
import { store as style } from '!/style'

const styleStore = style()
</script>

<template>
  <div :class="{ dark: styleStore.darkMode }">
    <div class="bg-brownish-50 dark:bg-brownish-800 dark:text-brownish-100">
      <slot />
    </div>
  </div>
</template>
