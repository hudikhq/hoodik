<script setup lang="ts">
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import { AppForm, AppField, AppButton } from '@/components/form'
import * as yup from 'yup'
import { store as registerStore } from '!/auth/register'
import { computed, ref } from 'vue'
import type { ErrorResponse } from '!/api'
import { notify } from '@kyvg/vue3-notification'

const register = registerStore()

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
      email: yup.string().email().required('Email is required')
    }),
    onSubmit: async (values: { email: string }, ctx: any) => {
      try {
        await register.resendActivation(values.email)
        notify("We've sent you an activation email")
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
        <h1 class="text-2xl text-white mb-5">Activation pending</h1>
        <div class="flex items-start">
          <div class="flex items-center h-5">
            <p class="text-sm dark:text-white">
              We have sent you an activation email that contains link you have to visit in order to
              active your account.<br />
              In case you haven't received an email, you can request another one by filling out the
              form below.
            </p>
          </div>
        </div>

        <AppForm :config="config" class="mt-8 space-y-6" v-slot="{ form }">
          <AppField :form="form" label="Your email" name="email" :autofocus="true" />

          <p v-if="resendError" class="text-sm text-redish-400">
            {{ resendError }}
          </p>

          <AppButton color="info" :form="form" type="submit" :disabled="disabled">
            <span v-if="disabled"> Re-send ({{ count }}) </span>
            <span v-else> Re-send </span>
          </AppButton>

          <div class="text-sm font-medium text-brownish-500 dark:text-brownish-400">
            Already activated an account?
            <router-link
              :to="{ name: 'login' }"
              class="text-primary-700 hover:underline dark:text-primary-500"
              >Login</router-link
            >
          </div>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
