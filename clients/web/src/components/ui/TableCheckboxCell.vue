<script setup lang="ts">
import { ref, watch } from 'vue'

defineProps<{
  type?: string
}>()

const emit = defineEmits(['checked'])

const checked = ref(false)

watch(checked, (newVal) => {
  emit('checked', newVal)
})
</script>

<template>
  <component :is="type || 'id'" class="lg:w-1">
    <label class="checkbox">
      <input v-model="checked" type="checkbox" />
      <span class="check" />
    </label>
  </component>
</template>
