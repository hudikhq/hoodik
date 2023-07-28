<script lang="ts" setup>
import { computed } from 'vue'

const props = defineProps<{
  modelValue?: number
  label?: string
  disabled?: boolean
  max?: number
}>()

const emits = defineEmits(['update:modelValue'])

const model = computed({
  get() {
    return `${props.modelValue || 0}`
  },
  set(value) {
    emits('update:modelValue', parseInt(value))
  }
})

const max = computed(() => {
  return props.max || 100
})
</script>
<template>
  <div>
    <span class="sm:block" for="slider" v-if="label"> {{ label }} </span>
    <input
      type="range"
      class="transparent h-1.5 w-full cursor-pointer appearance-none rounded-lg border-transparent bg-brownish-200 disabled:bg-brownish-600"
      id="slider"
      v-model="model"
      :max="max"
      :disabled="!!props.disabled"
    />
  </div>
</template>
