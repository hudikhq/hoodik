<script setup lang="ts">
const props = defineProps<{
  type?: string
  class?: string
  modelValue: boolean
}>()

const emit = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
}>()

const check = (event: Event) => {
  // @ts-ignore
  emit('update:modelValue', !!event.target?.checked)
}
</script>

<template>
  <component :is="type || 'id'" :class="props.class || 'lg:w-1'">
    <label class="checkbox">
      <input :checked="modelValue" @change="check" type="checkbox" />
      <span class="check" />
    </label>
  </component>
</template>
