<script setup lang="ts">
import SliderInput from './SliderInput.vue'
import UniversalCheckbox from './UniversalCheckbox.vue'
import { formatSize } from '!/index'
import { computed } from 'vue'

const props = defineProps<{
  modelValue: number | undefined
  disabled?: boolean
  title: string
}>()

const emits = defineEmits(['update:modelValue'])

const model = computed({
  get() {
    return props.modelValue
  },
  set(value) {
    emits('update:modelValue', value)
  }
})

const displayQuota = computed(() => {
  return formatSize(model.value || 0)
})

const unlimitedQuota = computed({
  get() {
    return typeof model.value !== 'number'
  },
  set(value) {
    model.value = value ? undefined : 0
  }
})
</script>
<template>
  <h3 class="text-lg mt-4">{{ title }}</h3>
  <UniversalCheckbox
    label="Unlimited"
    name="unlimited_quota"
    v-model="unlimitedQuota"
    :disabled="disabled"
  />

  <div class="flex flex-col mb-4" v-if="!unlimitedQuota">
    <div class="">
      <SliderInput v-model="model" :max="1024 * 1024 * 1024 * 1024" />
    </div>
    <div class="flex w-2/12">
      {{ displayQuota }}
    </div>
  </div>
</template>
