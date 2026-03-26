<script lang="ts" setup>
import CardBox from '@/components/ui/CardBox.vue'
import CardBoxComponentHeader from '@/components/ui/CardBoxComponentHeader.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { testEmail } from '!/admin/settings'
import { ref } from 'vue'
import { notify } from '@kyvg/vue3-notification'
import type { ErrorResponse } from '!/api'
import { mdiEmailCheckOutline } from '@mdi/js'

const props = defineProps<{
  loading: boolean
  class?: string
}>()

const testingEmail = ref(false)
const testError = ref<string>()
const testSuccess = ref<string>()

const sendTestEmail = async () => {
  testingEmail.value = true
  testError.value = undefined
  testSuccess.value = undefined

  try {
    const response = await testEmail()
    testSuccess.value = response.message
    notify({ text: response.message, type: 'success' })
  } catch (err) {
    const error = err as ErrorResponse<unknown>
    testError.value = error.description || 'Failed to send test email'
    notify({ text: testError.value, type: 'error' })
  }

  testingEmail.value = false
}
</script>
<template>
  <CardBox :class="props.class">
    <CardBoxComponentHeader title="Email Settings" />

    <div class="space-y-4 pt-2">
      <p class="text-sm text-brownish-400 dark:text-brownish-500 leading-relaxed">
        Verify your SMTP configuration by sending a test email to your account address.
      </p>

      <div v-if="testSuccess" class="rounded-lg bg-greeny-500/10 border border-greeny-500/30 px-4 py-3">
        <p class="text-sm text-greeny-500 dark:text-greeny-400">{{ testSuccess }}</p>
      </div>

      <div v-if="testError" class="rounded-lg bg-redish-500/10 border border-redish-500/30 px-4 py-3">
        <p class="text-sm text-redish-400">{{ testError }}</p>
      </div>

      <div class="flex items-center gap-3">
        <BaseButton
          color="info"
          :disabled="props.loading || testingEmail"
          @click="sendTestEmail"
          :icon="mdiEmailCheckOutline"
          :label="testingEmail ? 'Sending…' : 'Send Test Email'"
        />
        <span v-if="testingEmail" class="text-xs text-brownish-400 animate-pulse">Sending to your address…</span>
      </div>

      <p class="text-xs text-brownish-400 dark:text-brownish-500">
        The test email is sent to your registered account address.
      </p>
    </div>
  </CardBox>
</template>
