<script setup lang="ts">
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import SearchModalResult from '@/components/files/search/SearchModalResult.vue'
import { AppForm } from '@/components/form'
import { search } from '!/storage'
import { computed, ref, watch } from 'vue'
import * as yup from 'yup'
import type { KeyPair, ListAppFile } from 'types'
import { Field } from 'vee-validate'

const props = defineProps<{ keypair: KeyPair; modelValue: boolean }>()
const emits = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
}>()

const active = computed({
  get: () => props.modelValue,
  set: (value: boolean) => emits('update:modelValue', value)
})

const searchField = ref()
const results = ref<ListAppFile[]>([])
const searched = ref(false)
const config = ref()
const form = ref()

const focus = () => {
  setTimeout(() => {
    searchField.value?.focus()
  }, 1)
}

const init = () => {
  config.value = {
    initialValues: {
      query: ''
    },
    validationSchema: yup.object().shape({
      query: yup.string()
    }),
    onSubmit: async (values: { query: string }) => {
      searched.value = true
      focus()

      try {
        results.value = await search(values.query, props.keypair)
      } catch (err) {
        console.error(err)
      }
    }
  }
}

init()

watch(
  () => active.value,
  (value: boolean) => {
    if (value) {
      focus()
    } else if (typeof form.value?.form?.resetForm === 'function') {
      form.value?.form?.resetForm()
    }
  }
)
</script>
<template>
  <CardBoxModal v-model="active" :has-cancel="false" :hide-submit="true" @cancel="active = false">
    <AppForm ref="form" class="w-full" v-if="config" :config="config" v-slot="{ form, debounced }">
      <Field v-model="form.values.query" name="query" v-slot="{ field }">
        <input
          type="text"
          autofocus
          ref="searchField"
          v-bind="field"
          class="w-full px-4 py-2 text-gray-900 placeholder-gray-400 transition duration-150 ease-in-out bg-white border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500"
          @input="debounced"
        />
      </Field>
    </AppForm>

    <div
      class="w-full max-h-60 overflow-y-scroll mt-4 text-center"
      v-if="searched && !results.length"
    >
      No results
    </div>
    <div class="w-full max-h-60 overflow-y-scroll mt-4" v-else>
      <SearchModalResult
        v-for="file in results"
        :key="file.id"
        :file="file"
        @clicked="active = false"
      />
    </div>
  </CardBoxModal>
</template>