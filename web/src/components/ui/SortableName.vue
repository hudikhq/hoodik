<script lang="ts" setup>
import BaseIcon from './BaseIcon.vue'
import { mdiArrowDown, mdiArrowUp } from '@mdi/js'
const props = defineProps<{
  name: string
  label: string
  sortOptions: { parameter: string; order: string }
}>()

const emits = defineEmits<{
  (event: 'sort', order: string): void
}>()

const set = () => {
  if (props.sortOptions.parameter === props.name) {
    emits('sort', props.sortOptions.order === 'asc' ? `${props.name}|desc` : `${props.name}|asc`)
  } else {
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
      v-if="name === props.sortOptions.parameter"
      :path="props.sortOptions.order === 'asc' ? mdiArrowDown : mdiArrowUp"
      :size="23"
      class="ml-1"
    />
  </button>
</template>
