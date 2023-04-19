<script setup lang="ts">
import { ref, watch } from 'vue'

const props = defineProps<{
  type?: string
  class?: string
}>()

const emit = defineEmits(['checked'])

const checked = ref(false)

watch(checked, (newVal) => {
  emit('checked', newVal)
})
</script>

<template>
  <component :is="type || 'id'" :class="props.class || 'lg:w-1'">
    <label class="checkbox">
      <input v-model="checked" type="checkbox" />
      <span class="check" />
    </label>
  </component>
</template>
