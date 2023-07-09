<script setup lang="ts">
import type { UnsecureChangePassword, KeyPair } from 'types'
import { ref } from 'vue'
import { AppForm, AppField, AppButton, AppCheckbox } from '@/components/form'
import * as yup from 'yup'
import { changePassword } from '!/account'
import type { ErrorResponse } from '!/api'
import * as logger from '!/logger'
import { zxcvbn } from '@zxcvbn-ts/core'
import { notify } from '@kyvg/vue3-notification'

const props = defineProps<{
  keypair?: KeyPair
  forgotPassword?: boolean
  email?: string
}>()

const config = ref()
const changePasswordError = ref<string | null>(null)

const init = () => {
  config.value = {
    initialValues: {
      email: props.email || '',
      use_private_key: !!props.forgotPassword,
      private_key: '',
      current_password: '',
      password: '',
      token: ''
    } as UnsecureChangePassword,
    validationSchema: yup.object().shape({
      email: yup.string().required('Email is required').email('Email is invalid'),
      password: yup
        .string()
        .required('New password is required')
        .test(
          'weak-password',
          'New password used is too weak',
          (value: string) => zxcvbn(value).score > 3
        )
    }),
    onSubmit: async (values: UnsecureChangePassword, ctx: any) => {
      logger.debug('ChangePasswordForm: onSubmit', values)
      if (typeof values.token !== 'undefined' && !values.token) {
        delete values.token
      }

      if (!values.use_private_key) {
        if (!props.keypair || !props.keypair.input) {
          throw new Error('Missing keypair')
        }

        values.private_key = props.keypair.input
      } else if (typeof values.current_password !== 'undefined') {
        delete values.current_password
      }

      try {
        await changePassword(values)
        ctx.resetForm()

        notify('Your password has been changed')
      } catch (err) {
        const error = err as ErrorResponse<unknown>
        config.value.initialErrors = error.validation || {}
        changePasswordError.value = error.description || error.message

        if (error.validation) {
          config.value.initialErrors = error.validation
        }
      }
    }
  }
}

init()
</script>
<template>
  <AppForm v-if="config" :config="config" class="mt-8 space-y-6" v-slot="{ form }">
    <AppField
      v-if="forgotPassword"
      :form="form"
      label="Your email"
      name="email"
      placeholder="your@email.com"
      autofocus
      help="Enter the email address associated with your account."
    />

    <AppCheckbox
      v-if="!forgotPassword"
      label="Change with private key"
      :form="form"
      name="use_private_key"
    />

    <AppField
      v-if="form.values.use_private_key"
      textarea
      :rows="10"
      :form="form"
      label="Your private key"
      name="private_key"
      placeholder="Paste your private key here"
      help="Use your private key to sign the new password in order to authenticate you."
    />

    <AppField
      v-else
      :form="form"
      label="Your current password"
      name="current_password"
      type="password"
      help="Enter your current account password in order to authenticate you."
    />

    <div class="w-1/2 sm:w-1/4">
      <AppField
        type="password"
        :form="form"
        label="2FA token (optional)"
        name="token"
        placeholder="* * * * * *"
        class-add="text-sm"
        help="If you have 2FA enabled, enter your token here."
      />
    </div>

    <div class="border-2 p-2 pb-6 rounded-lg border-brownish-700">
      <AppField
        type="password"
        :form="form"
        label="New password"
        name="password"
        :disabled="!form.values.current_password && !form.values.private_key"
        help="Enter a new password that will be used to login to your account."
      />
    </div>

    <p v-if="changePasswordError" class="text-sm text-redish-400">
      {{ changePasswordError }}
    </p>

    <AppButton color="info" :form="form" type="submit">Change password</AppButton>
  </AppForm>
</template>
