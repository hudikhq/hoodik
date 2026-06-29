<script setup lang="ts">
import { computed } from 'vue'
import type { ShareRole } from 'types'

const props = withDefaults(
  defineProps<{
    modelValue: ShareRole
    testidPrefix: string
    disabled?: boolean
    disableCoOwner?: boolean
  }>(),
  {
    disabled: false,
    disableCoOwner: false
  }
)

const emit = defineEmits<{
  (e: 'update:modelValue', value: ShareRole): void
}>()

function chipClass(value: ShareRole, muted = false): string {
  const base =
    'relative inline-flex items-center justify-center min-h-[2.5rem] px-2 py-1.5 rounded-lg border text-sm cursor-pointer transition-colors select-none'
  const active =
    'border-redish-500 bg-redish-500 text-white font-medium'
  const inactive =
    'border-brownish-200 dark:border-brownish-700 text-brownish-700 dark:text-brownish-200 hover:border-brownish-300 dark:hover:border-brownish-500'
  const mutedClass = muted ? ' opacity-50 cursor-not-allowed' : ''
  return `${base} ${props.modelValue === value ? active : inactive}${mutedClass}`
}

const selected = computed({
  get: () => props.modelValue,
  set: (value: ShareRole) => emit('update:modelValue', value)
})
</script>

<template>
  <div class="grid grid-cols-3 gap-1.5">
    <label :class="chipClass('reader')">
      <input
        type="radio"
        v-model="selected"
        value="reader"
        class="absolute inset-0 w-full h-full opacity-0 cursor-pointer disabled:cursor-not-allowed z-10"
        :disabled="disabled"
        :data-testid="`${testidPrefix}-reader`"
      />
      <span>Reader</span>
    </label>
    <label :class="chipClass('editor')">
      <input
        type="radio"
        v-model="selected"
        value="editor"
        class="absolute inset-0 w-full h-full opacity-0 cursor-pointer disabled:cursor-not-allowed z-10"
        :disabled="disabled"
        :data-testid="`${testidPrefix}-editor`"
      />
      <span>Editor</span>
    </label>
    <label :class="chipClass('co-owner', disableCoOwner)">
      <input
        type="radio"
        v-model="selected"
        value="co-owner"
        class="absolute inset-0 w-full h-full opacity-0 cursor-pointer disabled:cursor-not-allowed z-10"
        :disabled="disabled || disableCoOwner"
        :data-testid="`${testidPrefix}-coowner`"
      />
      <span>Co-owner</span>
    </label>
  </div>
</template>
