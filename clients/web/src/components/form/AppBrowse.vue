<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  name: string
  class?: string | { [key: string]: boolean }
}>()

const emit = defineEmits(['change'])

const selected = ref()

const addedNewFile = function () {
  emit('change', selected.value.files)
}
</script>
<template>
  <div></div>
</template>
