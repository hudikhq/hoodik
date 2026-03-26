<script setup lang="ts">
import type { FormType } from '.'
import { computed, ref } from 'vue'
import { Field, ErrorMessage } from 'vee-validate'
import useClipboard from 'vue-clipboard3'
const { toClipboard } = useClipboard()

const originalClass =
  'w-full px-4 py-2 transition duration-150 ease-in-out rounded-lg ' +
  'bg-white dark:bg-brownish-800 ' +
  'border border-brownish-50 dark:border-brownish-600 ' +
  'text-brownish-900 dark:text-white ' +
  'placeholder-brownish-100/60 dark:placeholder-brownish-400 ' +
  'focus:outline-none focus:ring-2 focus:ring-offset-0 ' +
  'focus:ring-redish-400/60 dark:focus:ring-redish-500/50 ' +
  'focus:border-redish-400/50 dark:focus:border-redish-500/40'

const props = defineProps<{
  name: string
  form?: FormType
  type?: 'text' | 'password' | undefined
  label?: string | undefined
  allowCopy?: boolean | undefined
  placeholder?: string | undefined
  disabled?: boolean | undefined
  modelValue?: string | undefined
  required?: boolean | undefined
  error?: string
  textarea?: boolean
  rows?: number
  cols?: number
  class?: string
  classAdd?: string
  wrapperClass?: string
  help?: string
  autofocus?: boolean
  noOuterMargin?: boolean
}>()

const input = ref(null)

defineExpose({ input })

const emit = defineEmits(['update:modelValue', 'change', 'confirm'])

function change(e: Event) {
  if (props.form) {
    props.form.setFieldValue(props.name, (e.target as HTMLInputElement).value)
    props.form.validate()
  }

  emit('change', (e.target as HTMLInputElement).value)
}

function update(e: Event) {
  if (props.form) {
    props.form.setFieldValue(props.name, (e.target as HTMLInputElement).value)
  }

  emit('update:modelValue', (e.target as HTMLInputElement).value)
  emit('change', (e.target as HTMLInputElement).value)
}

function confirm(e: Event) {
  emit('confirm', (e.target as HTMLInputElement).value)
}

const model = computed<string>({
  get: () => {
    if (props.form) {
      return props.form.values[props.name]
    }
    return props.modelValue || ''
  },
  set: (v) => emit('update:modelValue', v)
})

const componentClass = ref<string>(
  `${props.class ? props.class : (props.classAdd || '') + ' ' + originalClass}`
)

const wrapClass = computed(
  () => `${props.noOuterMargin ? '' : 'mb-6 last:mb-0'} ${props.wrapperClass || ''}`
)

const copied = ref(false)

const copy = () => {
  toClipboard(model.value)
  copied.value = true
  setTimeout(() => {
    copied.value = false
  }, 2000)
}
</script>

<template>
  <div :class="wrapClass">
    <div class="w-full block">
      <div class="float-left" :class="{ 'w-1/2': allowCopy }" v-if="label">
        <label :for="name"> {{ label }} </label>
      </div>
      <div class="float-right w-1/2 mb-2" v-if="allowCopy">
        <button
          class="float-right text-center justify-center text-xs text-brownish-50 dark:text-brownish-400"
          :class="{ 'text-greeny-400 dark:text-greeny-300': copied }"
          @click.prevent="copy"
        >
          {{ copied ? 'Saved in clipboard' : 'Copy to clipboard' }}
        </button>
      </div>
    </div>
    <div :class="wrapperClass">
      <Field
        :id="name"
        :rows="rows"
        :cols="cols"
        v-if="textarea"
        as="textarea"
        v-model="model"
        :name="name"
        :disabled="disabled || form?.isSubmitting.value"
        ref="input"
        @input="update"
        @change="change"
        @blur="change"
        @keyup.enter="update"
        :class="componentClass"
      >
      </Field>
      <Field v-else v-model="model" :name="name" v-slot="{ field }">
        <input
          ref="input"
          :id="name"
          v-bind="field"
          @input="change"
          @change="change"
          :type="type || 'text'"
          :placeholder="placeholder || ''"
          :disabled="disabled || form?.isSubmitting.value"
          :autofocus="!!props.autofocus"
          :class="componentClass"
          @keyup.enter="confirm"
        />
      </Field>
    </div>
    <div v-if="help" class="text-xs text-brownish-100 dark:text-brownish-400 mt-1">
      {{ help }}
    </div>

    <ErrorMessage
      :name="name"
      class="block text-sm text-redish-700 dark:text-redish-500 ml-2 mb-[-1.25rem]"
    />
  </div>
</template>
