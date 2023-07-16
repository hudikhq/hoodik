<script lang="ts" setup>
import type { Authenticated } from 'types/login'
import { store as registerStore } from '!/auth/register'
import type { ErrorResponse } from '!/api'
import { notify } from '@kyvg/vue3-notification'

const register = registerStore()

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
        title: "Couldn't resend activation email",
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
    You account is not activated, please check your email for the activation link, it might end up
    in spam folder, so check that too. You can also try to
    <a class="underline hover:no-underline" href="#" @click.prevent="resend">resend</a> the
    activation email.
  </div>
</template>
