<script setup lang="ts">
import { computed, ref, onMounted, onBeforeUnmount } from 'vue'
import { mdiFileSearch } from '@mdi/js'
import BaseIcon from './BaseIcon.vue'

const props = defineProps({
  modelValue: {
    type: [String, Number, Boolean, Array, Object],
    default: ''
  }
})

const emit = defineEmits(['update:modelValue', 'setRef'])

const computedValue = computed({
  get: () => props.modelValue,
  set: (value) => {
    emit('update:modelValue', value)
  }
})

const inputElClass = computed(() => {
  const base = [
    'px-3 py-2 max-w-full focus:ring focus:outline-none border-gray-700 rounded w-full',
    'dark:placeholder-gray-400',
    'h-12',
    'border-0',
    'bg-transparent',
    'pl-10'
  ]

  return base
})

const inputEl = ref()

const placeholder = ref('ctrl + k')

onMounted(() => {
  emit('setRef', inputEl.value)
})

const fieldFocusHook = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
    e.preventDefault()

    if (inputEl.value) {
      inputEl.value.focus()
    }
  } else if (e.key === 'Escape') {
    if (inputEl.value) {
      inputEl.value.blur()
    }
    computedValue.value = ''
  }
}

onMounted(() => {
  window.addEventListener('keydown', fieldFocusHook)
})

onBeforeUnmount(() => {
  window.removeEventListener('keydown', fieldFocusHook)
})
</script>

<template>
  <div class="relative">
    <input
      id="search-bar"
      ref="inputEl"
      v-model="computedValue"
      :class="inputElClass"
      :placeholder="placeholder"
      name="Search"
      autocomplete="false"
      inputmode="text"
      type="text"
    />
    <BaseIcon
      :path="mdiFileSearch"
      w="w-10"
      h="h-12"
      class="absolute top-0 left-0 z-10 pointer-events-none text-gray-500 dark:text-slate-400"
    />
  </div>
</template>
