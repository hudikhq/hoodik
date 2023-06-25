<script lang="ts" setup>
import { computed } from 'vue'
import BaseIcon from './BaseIcon.vue'
import { mdiArrowDown, mdiArrowUp } from '@mdi/js'
const props = defineProps<{
  name: string
  label: string
  sortOptions?: { sort?: string; parameter?: string; order?: string }
  modelValue?: { sort?: string; parameter?: string; order?: string }
}>()

const sortOptions = computed(() => {
  if (props.modelValue) return props.modelValue

  return props.sortOptions || { sort: null, parameter: null, order: null }
})

const emits = defineEmits<{
  (event: 'update:modelValue', value: { sort?: string; parameter?: string; order?: string }): void
  (event: 'sort', order: string): void
}>()

const sort = computed(() => {
  if (sortOptions.value.sort) return sortOptions.value.sort
  if (sortOptions.value.parameter) return sortOptions.value.parameter

  return null
})

const order = computed(() => {
  if (sortOptions.value.order) return sortOptions.value.order

  return 'asc'
})

const set = () => {
  if (sortOptions.value.parameter === props.name) {
    emits('sort', sortOptions.value.order === 'asc' ? `${props.name}|desc` : `${props.name}|asc`)
    emits('update:modelValue', {
      ...sortOptions.value,
      sort: props.name,
      parameter: props.name,
      order: sortOptions.value.order === 'asc' ? 'desc' : 'asc'
    })
  } else {
    emits('update:modelValue', {
      ...sortOptions.value,
      sort: props.name,
      parameter: props.name,
      order: 'asc'
    })
    emits('sort', `${props.name}|asc`)
  }
}
</script>
<template>
  <button
    @click="set"
    class="p-1 m-0 flex hover:bg-brownish-500 rounded-md"
    :name="label"
    :title="label"
  >
    <span class="inline-block">{{ label }}</span>
    <BaseIcon
      v-if="name === sort"
      :path="order === 'asc' ? mdiArrowDown : mdiArrowUp"
      :size="23"
      class="ml-1"
    />
  </button>
</template>
