<script setup lang="ts">
import type { FormType } from '.'
import { computed, nextTick, onMounted, ref } from 'vue'
import { ErrorMessage } from 'vee-validate'

const props = withDefaults(
  defineProps<{
    name: string
    form?: FormType
    label?: string
    length?: number
    autofocus?: boolean
    disabled?: boolean
  }>(),
  { length: 6 }
)

const emit = defineEmits<{
  (event: 'update:modelValue', value: string): void
  (event: 'complete', value: string): void
}>()

const digits = ref<string[]>(Array.from({ length: props.length }, () => ''))
const boxes = ref<HTMLInputElement[]>([])

const value = computed(() => digits.value.join(''))

const attempted = computed(() => (props.form?.submitCount?.value ?? 0) > 0)

const publish = () => {
  props.form?.setFieldValue(props.name, value.value)
  emit('update:modelValue', value.value)

  if (value.value.length === props.length) {
    emit('complete', value.value)
  }
}

const focusBox = (index: number) => {
  const box = boxes.value[Math.max(0, Math.min(index, props.length - 1))]
  box?.focus()
  box?.select()
}

/**
 * A password manager or an iOS SMS suggestion drops the whole code into
 * whichever box happened to have focus, so any input longer than one character
 * is spread across the row rather than truncated to its first digit.
 */
const onInput = (index: number, event: Event) => {
  const input = event.target as HTMLInputElement
  const typed = input.value.replace(/\D/g, '')

  if (!typed) {
    digits.value[index] = ''
    input.value = ''
    publish()
    return
  }

  const next = [...digits.value]
  ;[...typed].forEach((char, offset) => {
    if (index + offset < props.length) next[index + offset] = char
  })
  digits.value = next

  // The DOM value can hold the whole pasted string; the box shows one digit.
  input.value = digits.value[index]

  publish()
  focusBox(index + typed.length)
}

const onKeydown = (index: number, event: KeyboardEvent) => {
  if (event.key === 'Backspace') {
    if (digits.value[index]) {
      digits.value[index] = ''
      publish()
      return
    }
    // Already empty — clear the one behind and go there, so holding backspace
    // walks the row instead of stalling.
    event.preventDefault()
    if (index > 0) {
      digits.value[index - 1] = ''
      publish()
      focusBox(index - 1)
    }
    return
  }

  if (event.key === 'ArrowLeft') {
    event.preventDefault()
    focusBox(index - 1)
  } else if (event.key === 'ArrowRight') {
    event.preventDefault()
    focusBox(index + 1)
  }
}

const onPaste = (index: number, event: ClipboardEvent) => {
  const pasted = event.clipboardData?.getData('text')?.replace(/\D/g, '')
  if (!pasted) return

  event.preventDefault()
  const next = [...digits.value]
  ;[...pasted].forEach((char, offset) => {
    if (index + offset < props.length) next[index + offset] = char
  })
  digits.value = next
  publish()
  focusBox(index + pasted.length)
}

const clear = () => {
  digits.value = Array.from({ length: props.length }, () => '')
  publish()
  focusBox(0)
}

defineExpose({ clear })

onMounted(() => {
  if (props.autofocus) nextTick(() => focusBox(0))
})
</script>

<template>
  <div>
    <label v-if="label" :for="`${name}-0`" class="block mb-2">{{ label }}</label>

    <div class="flex w-full gap-2 sm:gap-3" role="group" :aria-label="label">
      <input
        v-for="(digit, index) in digits"
        :key="index"
        :ref="(el) => { if (el) boxes[index] = el as HTMLInputElement }"
        :id="index === 0 ? `${name}-0` : undefined"
        :value="digit"
        type="text"
        inputmode="numeric"
        maxlength="1"
        autocomplete="one-time-code"
        :aria-label="$t('auth.login.codeDigit', { index: index + 1, total: length })"
        :disabled="disabled || form?.isSubmitting.value"
        class="flex-1 min-w-0 h-16 text-center text-2xl font-medium tabular-nums rounded-lg
               bg-white dark:bg-brownish-800
               border text-brownish-900 dark:text-white
               transition duration-150 ease-in-out
               focus:outline-none focus:ring-2 focus:ring-offset-0
               focus:ring-redish-400/60 dark:focus:ring-redish-500/50
               focus:border-redish-400/50 dark:focus:border-redish-500/40
               disabled:opacity-50"
        :class="
          digit
            ? 'border-brownish-200 dark:border-brownish-500'
            : 'border-paper-300 dark:border-brownish-600'
        "
        @input="onInput(index, $event)"
        @keydown="onKeydown(index, $event)"
        @paste="onPaste(index, $event)"
        @focus="($event.target as HTMLInputElement).select()"
      />
    </div>

    <!-- Nobody has failed to enter a code they have not reached yet, so the
         required message waits for an actual submit attempt. -->
    <ErrorMessage
      v-if="attempted"
      :name="name"
      class="block mt-2 text-sm text-redish-700 dark:text-redish-100"
    />
  </div>
</template>
