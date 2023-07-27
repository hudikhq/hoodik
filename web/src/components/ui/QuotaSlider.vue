<script setup lang="ts">
import SliderInput from './SliderInput.vue'
import UniversalCheckbox from './UniversalCheckbox.vue'
import { formatSize } from '!/index'
import { computed } from 'vue'

const props = defineProps<{
  modelValue: number | undefined
  disabled?: boolean
  title?: string
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

const defaultQuota = computed({
  get() {
    return typeof model.value !== 'number'
  },
  set(value) {
    model.value = value ? undefined : 0
  }
})
</script>
<template>
  <h3 class="text-lg mt-4" v-if="title">{{ title }}</h3>
  <div class="flex w-full">
    <div class="pr-2">
      <UniversalCheckbox
        :label="!defaultQuota ? displayQuota : 'default'"
        name="default_quota"
        v-model="defaultQuota"
        :disabled="disabled"
      />
    </div>
    <div class="w-full">
      <SliderInput :disabled="defaultQuota" v-model="model" :max="1024 * 1024 * 1024 * 1024" />
    </div>
  </div>
</template>
