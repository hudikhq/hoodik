<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  name: string
  label: string | undefined
  disabled?: boolean | undefined
  modelValue: boolean | undefined
}>()

const emit = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
}>()

const model = computed<boolean | undefined>({
  get() {
    return props.modelValue
  },
  set(value) {
    emit('update:modelValue', !!value)
  }
})
</script>

<template>
  <div class="flex-nowrap">
    <label class="checkbox">
      <input :id="name" type="checkbox" v-model="model" :disabled="disabled" />
      <span class="check" />
      <span class="pl-2 w-24 sm:block">
        {{ label }}
      </span>
    </label>
  </div>
</template>
