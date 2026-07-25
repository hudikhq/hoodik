<script setup lang="ts">
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import { AppForm, AppField, AppButton } from '@/components/form'
import * as yup from 'yup'
import { store as registerStore } from '!/auth/register'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ErrorResponse } from '!/api'
import { notify } from '@kyvg/vue3-notification'

const register = registerStore()
const { t } = useI18n()

const config = ref()
const resendError = ref<string | null>(null)

const count = ref(60)

setInterval(async () => {
  count.value = count.value - 1
}, 1000)

const disabled = computed(() => {
  return count.value > 0
})

const init = () => {
  config.value = {
    initialValues: {
      email: ''
    },
    validationSchema: yup.object().shape({
      email: yup.string().email().required(t('auth.validation.emailRequired'))
    }),
    onSubmit: async (values: { email: string }, ctx: any) => {
      try {
        await register.resendActivation(values.email)
        notify(t('auth.resend.sentNotification'))
        ctx.resetForm()
        count.value = 60
      } catch (err) {
        const error = err as ErrorResponse<unknown>
        ctx.setErrors(error.validation || {})
        resendError.value = error.description
        count.value = 60
      }
    }
  }
}

init()
</script>
<template>
  <LayoutGuest>
    <SectionFullScreen v-slot="{ cardClass }" bg="pinkRed">
      <CardBox :class="cardClass" v-if="config">
        <h1 class="text-2xl text-white mb-5">{{ $t('auth.resend.title') }}</h1>
        <div class="flex items-start">
          <div class="flex items-center h-5">
            <p class="text-sm dark:text-white">
              {{ $t('auth.resend.description1') }}<br />
              {{ $t('auth.resend.description2') }}
            </p>
          </div>
        </div>

        <AppForm :config="config" class="mt-8 space-y-6" v-slot="{ form }">
          <AppField :form="form" :label="$t('auth.yourEmail')" name="email" :autofocus="true" />

          <p v-if="resendError" class="text-sm text-redish-400">
            {{ resendError }}
          </p>

          <AppButton color="info" :form="form" type="submit" :disabled="disabled">
            <span v-if="disabled"> {{ $t('auth.resend.resendCount', { count }) }} </span>
            <span v-else> {{ $t('auth.resend.resend') }} </span>
          </AppButton>

          <div class="text-sm font-medium text-brownish-500 dark:text-brownish-100">
            {{ $t('auth.resend.alreadyActivated') }}
            <router-link
              :to="{ name: 'login' }"
              class="text-primary-700 hover:underline dark:text-primary-500"
              >{{ $t('common.login') }}</router-link
            >
          </div>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
