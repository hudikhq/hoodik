<script lang="ts" setup>
import { index, update as updateInner } from '!/admin/settings'
import type { Data } from 'types/admin/settings'
import { ref } from 'vue'
import UserSettings from './UserSettings.vue'
import EmailSettings from './EmailSettings.vue'
import type { ErrorResponse } from '!/api'
import { notify } from '@kyvg/vue3-notification'

const loading = ref(false)
const settings = ref<Data>()
const updateError = ref<string>()

const init = async () => {
  settings.value = await index()
}

const save = async () => {
  if (!settings.value) return

  loading.value = true
  updateError.value = undefined

  try {
    settings.value = await updateInner(settings.value)
    notify({ text: 'Settings saved', type: 'success' })
  } catch (err) {
    const error = err as ErrorResponse<unknown>
    updateError.value = error.description
  }

  loading.value = false
}

await init()
</script>
<template>
  <div class="mb-8">
    <h2 class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-500 mb-3 px-1">Application Settings</h2>
    <div class="flex flex-col lg:flex-row gap-6">
      <UserSettings
        class="w-full lg:w-7/12"
        v-model="settings"
        :loading="loading"
        :error="updateError"
        @save="save"
      />
      <EmailSettings class="w-full lg:w-5/12" :loading="loading" />
    </div>
  </div>
</template>
