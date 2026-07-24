<script lang="ts" setup>
import type { Authenticated } from 'types/login'
import { store as registerStore } from '!/auth/register'
import type { ErrorResponse } from '!/api'
import { notify } from '@kyvg/vue3-notification'
import { useI18n } from 'vue-i18n'

const register = registerStore()
const { t } = useI18n()

const props = defineProps<{
  authenticated?: Authenticated | null
}>()

const resend = async () => {
  if (props.authenticated?.user?.email) {
    try {
      await register.resendActivation(props.authenticated.user.email)
    } catch (err) {
      const error = err as ErrorResponse<void>

      notify({
        title: t('nav.activation.resendFailed'),
        text: error.description,
        type: 'error'
      })
    }
  }
}
</script>
<template>
  <div
    v-if="authenticated && !authenticated?.user?.email_verified_at"
    class="block bg-redish-100 dark:bg-redish-950 text-redish-950 dark:text-redish-100 rounded-lg p-4 mx-1 xl:mx-6"
  >
    <i18n-t keypath="nav.activation.notice" scope="global">
      <template #resend>
        <a class="underline hover:no-underline" href="#" @click.prevent="resend">
          {{ $t('nav.activation.resend') }}
        </a>
      </template>
    </i18n-t>
  </div>
</template>
