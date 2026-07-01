<script setup lang="ts">
const props = defineProps<{
  type?: string
  class?: string
  modelValue: boolean
  disabled?: boolean
}>()

const emit = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
}>()

const check = (event: Event) => {
  if (props.disabled) return
  // @ts-ignore
  emit('update:modelValue', !!event.target?.checked)
}
</script>

<template>
  <component :is="type || 'span'" :class="props.class || 'lg:w-1'">
    <label class="checkbox" :class="{ 'opacity-40 cursor-not-allowed': disabled }">
      <input :checked="modelValue" :disabled="disabled" @change="check" type="checkbox" />
      <span class="check" />
    </label>
  </component>
</template>
