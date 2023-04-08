<script setup lang="ts">
import { Field, ErrorMessage } from 'vee-validate'
import type { FormType } from '.'
import { computed } from 'vue'

const props = defineProps<{
  name: string
  label: string | undefined
  allowCopy?: boolean | undefined
  placeholder?: string | undefined
  disabled?: boolean | undefined
  form: FormType
  required?: boolean | undefined
  error?: string
  type?: 'radio' | 'checkbox' | undefined
}>()

const classType = computed(() => {
  return props.type === 'radio' ? 'radio' : 'checkbox'
})

const onChange = (e: Event) => {
  props.form.setFieldValue(props.name, (e.target as HTMLInputElement).checked)
}
</script>

<template>
  <div>
    <label
      :class="{
        [classType]: true
      }"
    >
      <Field
        :name="name"
        v-slot="{ field }"
        type="checkbox"
        :value="form.values[name]"
        @change="onChange"
      >
        <input :id="name" type="checkbox" v-bind="field" :checked="!!form.values[name]" />
      </Field>
      <span class="check" />
      <span class="pl-2">{{ label }}</span>
      <ErrorMessage :name="name" class="text-sm text-red-700 dark:text-red-500 ml-2" />
    </label>
  </div>
</template>
