<script setup lang="ts">
import type { FormType } from '.'
import { ref } from 'vue'
import { Field, ErrorMessage } from 'vee-validate'
import useClipboard from 'vue-clipboard3'
const { toClipboard } = useClipboard()

const originalClass =
  'w-full px-4 py-2 text-gray-900 placeholder-gray-400 transition duration-150 ease-in-out bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500'

const props = defineProps<{
  name: string
  form: FormType
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
}>()

const emit = defineEmits(['update:modelValue', 'change'])

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
}

const model = ref<string>(props.form.values[props.name] || props.modelValue || '')

const componentClass = ref<string>(
  `${props.class ? props.class : (props.classAdd || '') + ' ' + originalClass}`
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
  <div class="mb-6 last:mb-0">
    <div class="w-full block">
      <div class="float-left" :class="{ 'w-1/2': allowCopy }" v-if="label">
        <label :for="name"> {{ label }} </label>
      </div>
      <div class="float-right w-1/2 mb-2" v-if="allowCopy">
        <button
          class="float-right text-center justify-center text-xs text-gray-400"
          :class="{ 'text-green-600': copied }"
          @click.prevent="copy"
        >
          {{ copied ? 'Saved in clipboard' : 'Copy to clipboard' }}
        </button>
      </div>
    </div>
    <div :class="wrapperClass">
      <Field v-model="model" :name="name" v-slot="{ field }">
        <textarea
          v-if="textarea"
          :id="name"
          :rows="rows"
          :cols="cols"
          v-bind="field"
          @input="update"
          @change="change"
          @blur="change"
          @keyup.enter="update"
          :class="componentClass"
          :type="type || 'text'"
          :placeholder="placeholder || ''"
          :disabled="disabled || form.isSubmitting.value"
        ></textarea>
        <input
          v-else
          :id="name"
          v-bind="field"
          @input="update"
          @change="change"
          @blur="change"
          @keyup.enter="update"
          :class="componentClass"
          :type="type || 'text'"
          :placeholder="placeholder || ''"
          :disabled="disabled || form.isSubmitting.value"
        />
      </Field>
    </div>
    <div v-if="help" class="text-xs text-gray-500 dark:text-slate-400 mt-1">
      {{ help }}
    </div>
    <ErrorMessage :name="name" class="text-xs text-red-700 dark:text-red-500" />
  </div>
</template>
