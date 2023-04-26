<script setup lang="ts">
const props = defineProps({
  navBar: Boolean
})
</script>

<template>
  <hr
    :class="
      props.navBar
        ? 'hidden lg:block lg:my-0.5 dark:border-brownish-700'
        : 'my-6 -mx-6 dark:border-brownish-800'
    "
    class="border-t border-brownish-100"
  />
</template>
