<script lang="ts" setup>
import { index, update as updateInner } from '!/admin/settings'
import type { Data } from 'types/admin/settings'
import { ref } from 'vue'
import UserSettings from './UserSettings.vue'
import EmailSettings from './EmailSettings.vue'
import CardBoxComponentFooter from '@/components/ui/CardBoxComponentFooter.vue'
import type { ErrorResponse } from '!/api'
import BaseButton from '@/components/ui/BaseButton.vue'
import { notify } from '@kyvg/vue3-notification'

const loading = ref(false)
const settings = ref<Data>()
const updateError = ref()

const init = async () => {
  settings.value = await index()
}

const update = async () => {
  if (!settings.value) {
    return
  }

  loading.value = true
  updateError.value = undefined

  try {
    settings.value = await updateInner(settings.value)
    notify('Settings updated')
  } catch (err) {
    const error = err as ErrorResponse<unknown>
    updateError.value = error.description
  }

  loading.value = false
}

await init()
</script>
<template>
  <p v-if="updateError" class="text-sm text-redish-400">
    {{ updateError }}
  </p>

  <UserSettings class="w-full sm:w-1/2" v-model="settings" :loading="loading" />

  <EmailSettings class="w-full sm:w-1/2 mt-6" :loading="loading" />

  <CardBoxComponentFooter>
    <BaseButton color="info" :disabled="loading" @click="update" label="Save" />
  </CardBoxComponentFooter>
</template>
