<script lang="ts" setup>
import CardBox from '@/components/ui/CardBox.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import { testEmail } from '!/admin/settings'
import { ref } from 'vue'
import { notify } from '@kyvg/vue3-notification'
import type { ErrorResponse } from '!/api'
import { mdiEmailCheckOutline, mdiEmail } from '@mdi/js'

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
    <!-- Email header -->
    <div class="-mx-4 -mt-4 px-6 py-5 border-b border-brownish-100 dark:border-brownish-700/50 rounded-t-2xl">
      <div class="flex items-center gap-2 mb-3">
        <BaseIcon :path="mdiEmail" :size="14" class="text-brownish-400 dark:text-brownish-500" />
        <p class="text-xs font-semibold uppercase tracking-wider text-brownish-400 dark:text-brownish-500">Email Configuration</p>
      </div>
      <p class="text-sm text-brownish-400 dark:text-brownish-500 leading-relaxed">
        Verify your SMTP configuration by sending a test email to your account address.
      </p>
    </div>

    <!-- Test email action -->
    <div class="-mx-4 -mb-4 px-6 py-5 rounded-b-2xl">
      <div v-if="testSuccess" class="rounded-lg bg-greeny-500/10 border border-greeny-500/30 px-4 py-3 mb-3">
        <p class="text-sm text-greeny-500 dark:text-greeny-400">{{ testSuccess }}</p>
      </div>

      <div v-if="testError" class="rounded-lg bg-redish-500/10 border border-redish-500/30 px-4 py-3 mb-3">
        <p class="text-sm text-redish-400">{{ testError }}</p>
      </div>

      <div class="p-3 rounded-xl bg-brownish-50/50 dark:bg-brownish-700/20 border border-brownish-100/50 dark:border-brownish-700/30">
        <div class="flex items-center justify-between gap-4">
          <div class="min-w-0">
            <p class="text-sm font-medium">Test Email</p>
            <p class="text-xs text-brownish-400 dark:text-brownish-500 mt-0.5">Sends to your registered account address</p>
          </div>
          <div class="flex items-center gap-3 shrink-0">
            <span v-if="testingEmail" class="text-xs text-brownish-400 animate-pulse">Sending…</span>
            <BaseButton
              color="info"
              :small="true"
              :disabled="props.loading || testingEmail"
              @click="sendTestEmail"
              :icon="mdiEmailCheckOutline"
              :label="testingEmail ? 'Sending…' : 'Send Test'"
            />
          </div>
        </div>
      </div>
    </div>
  </CardBox>
</template>
