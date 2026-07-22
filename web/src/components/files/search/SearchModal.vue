<script setup lang="ts">
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import SearchModalResult from '@/components/files/search/SearchModalResult.vue'
import { AppForm } from '@/components/form'
import { search } from '!/storage'
import { computed, ref, watch } from 'vue'
import * as yup from 'yup'
import type { KeyPair, AppFile } from 'types'
import { Field } from 'vee-validate'
import { mdiFileDocumentOutline, mdiFileSearchOutline, mdiMagnify } from '@mdi/js'
import BaseIcon from '@/components/ui/BaseIcon.vue'

const props = defineProps<{ keypair: KeyPair; modelValue: boolean }>()
const emits = defineEmits<{
  (event: 'update:modelValue', value: boolean): void
}>()

const active = computed({
  get: () => props.modelValue,
  set: (value: boolean) => emits('update:modelValue', value)
})

const searchField = ref()
const results = ref<AppFile[]>([])
const searched = ref(false)
const loading = ref(false)
const notesOnly = ref(false)
const config = ref()
const form = ref()

const focus = () => {
  setTimeout(() => {
    searchField.value?.focus()
  }, 1)
}

// Each keystroke fires its own request and they can resolve out of order, so
// only the newest one is allowed to write results — otherwise a slow early
// query lands after a fast later one and the list contradicts the input.
let latestSearch = 0

async function doSearch(query: string) {
  const ticket = ++latestSearch
  loading.value = true

  try {
    const found = await search(
      query,
      props.keypair,
      notesOnly.value ? { editable: true } : undefined
    )

    if (ticket !== latestSearch) return

    results.value = found
    searched.value = true
  } catch (err) {
    if (ticket === latestSearch) {
      results.value = []
      searched.value = true
    }
  } finally {
    // `searched` flips only once a request has actually come back: setting it
    // up front rendered "no matches" over every in-flight query.
    if (ticket === latestSearch) loading.value = false
  }
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
      focus()
      await doSearch(values.query)
    }
  }
}

init()

// Re-run search when toggling notes filter (if there's an active query)
watch(notesOnly, () => {
  const query = form.value?.form?.values?.query
  if (query && searched.value) {
    doSearch(query)
  }
})

watch(
  () => active.value,
  (value: boolean) => {
    if (value) {
      focus()
    } else if (typeof form.value?.form?.resetForm === 'function') {
      form.value?.form?.resetForm()
      results.value = []
      searched.value = false
      loading.value = false
      // Abandon anything still in flight so it can't repopulate a closed modal.
      latestSearch++
    }
  }
)
</script>
<template>
  <CardBoxModal v-model="active" :has-cancel="false" :hide-submit="true" @cancel="active = false">
    <AppForm ref="form" class="w-full" v-if="config" :config="config" v-slot="{ form, debounced }">
      <Field v-model="form.values.query" name="query" v-slot="{ field }">
        <div class="relative">
          <BaseIcon
            :path="mdiMagnify"
            :size="18"
            class="absolute inset-y-0 left-3 text-brownish-300 dark:text-brownish-100 pointer-events-none"
          />
          <input
            type="text"
            autofocus
            ref="searchField"
            v-bind="field"
            class="w-full pl-10 pr-4 py-2 transition duration-150 ease-in-out rounded-lg
              bg-white dark:bg-brownish-800
              border border-brownish-50 dark:border-brownish-600
              text-brownish-900 dark:text-white
              placeholder-brownish-100/60 dark:placeholder-brownish-400
              focus:outline-none focus:ring-2 focus:ring-offset-0
              focus:ring-redish-400/60 dark:focus:ring-redish-500/50
              focus:border-redish-400/50 dark:focus:border-redish-500/40"
            placeholder="Search files..."
            @input="debounced"
          />
        </div>
      </Field>

      <label class="flex items-center gap-2 mt-2 text-sm cursor-pointer text-brownish-400 dark:text-brownish-100 hover:text-brownish-600 dark:hover:text-brownish-50 transition-colors select-none">
        <input
          type="checkbox"
          v-model="notesOnly"
          class="rounded border-brownish-600 bg-brownish-800 text-orangy-500 focus:ring-orangy-500 focus:ring-offset-0"
        />
        <BaseIcon :path="mdiFileDocumentOutline" :size="14" />
        Notes only
      </label>
    </AppForm>

    <!-- Placeholders mirror the result row's shape, so the list settles into
         place instead of the panel resizing under the cursor. -->
    <div class="w-full mt-4" v-if="loading" aria-busy="true">
      <div
        v-for="(width, index) in ['w-3/5', 'w-4/5', 'w-2/5']"
        :key="index"
        class="flex items-center gap-3 p-2 animate-pulse"
      >
        <div class="w-10 h-10 rounded-md shrink-0 bg-brownish-50/50 dark:bg-brownish-700" />
        <div class="min-w-0 flex-1 space-y-2">
          <div class="h-3.5 rounded bg-brownish-50/50 dark:bg-brownish-700" :class="width" />
          <div class="h-3 w-20 rounded bg-brownish-50/40 dark:bg-brownish-700/60" />
        </div>
        <div class="h-3 w-12 rounded shrink-0 bg-brownish-50/40 dark:bg-brownish-700/60" />
      </div>
    </div>

    <!-- `max-h` rather than a fixed height: a single hit shouldn't sit above
         a wall of empty space, and `auto` keeps the scrollbar gutter from
         being painted when there's nothing to scroll. -->
    <div
      class="w-full mt-4 flex flex-col items-center justify-center gap-2 h-40 text-brownish-300 dark:text-brownish-100"
      v-else-if="!results.length"
    >
      <BaseIcon :path="searched ? mdiFileSearchOutline : mdiMagnify" :size="28" />
      <span class="text-sm">
        {{ searched ? 'No files match that search' : 'Start typing to search' }}
      </span>
    </div>
    <div class="w-full max-h-72 overflow-y-auto mt-4" v-else>
      <SearchModalResult
        v-for="file in results"
        :key="file.id"
        :file="file"
        @clicked="active = false"
      />
    </div>
  </CardBoxModal>
</template>
