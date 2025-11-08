<script lang="ts" setup>
import CardBox from '@/components/ui/CardBox.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { testEmail } from '!/admin/settings'
import { ref } from 'vue'
import { notify } from '@kyvg/vue3-notification'
import type { ErrorResponse } from '!/api'

const props = defineProps<{
  loading: boolean
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
  <CardBox>
    <h1 class="text-2xl text-white mb-4">Email Settings</h1>

    <p class="text-sm text-brownish-300 mb-4">
      Test your email configuration by sending a test email to your account. This will verify that
      your SMTP settings are configured correctly.
    </p>

    <div class="mb-4" v-if="testSuccess">
      <p class="text-sm text-greenish-400">
        {{ testSuccess }}
      </p>
    </div>

    <div class="mb-4" v-if="testError">
      <p class="text-sm text-redish-400">
        {{ testError }}
      </p>
    </div>

    <BaseButton
      color="info"
      :disabled="props.loading || testingEmail"
      @click="sendTestEmail"
      :label="testingEmail ? 'Sending...' : 'Send Test Email'"
    />

    <p class="text-xs text-brownish-400 mt-2">
      The test email will be sent to your registered email address.
    </p>
  </CardBox>
</template>

