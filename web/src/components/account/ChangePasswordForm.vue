<script setup lang="ts">
import type { UnsecureChangePassword, KeyPair } from 'types'
import { ref, computed } from 'vue'
import { AppForm, AppField, AppButton, AppCheckbox } from '@/components/form'
import * as yup from 'yup'
import { changePassword, changePasswordV2 } from '!/account'
import type { ErrorResponse } from '!/api'
import * as logger from '!/logger'
import { isStrongPassword } from '@/utils/password'
import { notify } from '@kyvg/vue3-notification'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  keypair?: KeyPair
  forgotPassword?: boolean
  email?: string
}>()

const { t } = useI18n()

const config = ref()
const changePasswordError = ref<string | null>(null)

// v2 accounts authenticate the change through their session + in-memory keys,
// so the legacy proof inputs are hidden. Not available in forgot-password mode
// (no session, no in-memory keypair).
const isCurve = computed(() => !props.forgotPassword && props.keypair?.keyType === 'curve25519')

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
      email: yup
        .string()
        .required(t('account.changePassword.emailRequired'))
        .email(t('account.changePassword.emailInvalid')),
      password: yup
        .string()
        .required(t('account.changePassword.passwordRequired'))
        .test(
          'weak-password',
          t('account.changePassword.passwordWeak'),
          (value: string | undefined) => isStrongPassword(value)
        )
    }),
    onSubmit: async (values: UnsecureChangePassword, ctx: any) => {
      logger.debug('ChangePasswordForm: onSubmit', values)
      if (typeof values.token !== 'undefined' && !values.token) {
        delete values.token
      }

      // v2 (Curve25519 + OPAQUE) accounts change the password through the PAKE
      // flow: the session already authenticates the user and the in-memory keys
      // re-seal the envelope, so none of the legacy proof inputs apply.
      if (!isCurve.value) {
        if (!values.use_private_key) {
          if (!props.keypair || !props.keypair.input) {
            throw new Error('Missing keypair')
          }

          values.private_key = props.keypair.input
        } else if (typeof values.current_password !== 'undefined') {
          delete values.current_password
        }
      }

      try {
        if (isCurve.value) {
          await changePasswordV2(props.keypair as KeyPair, values.password, values.token)
        } else {
          await changePassword(values)
        }
        ctx.resetForm()

        notify(t('account.changePassword.success'))
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
      :label="$t('account.changePassword.emailLabel')"
      name="email"
      placeholder="your@email.com"
      autocomplete="username"
      autofocus
      :help="$t('account.changePassword.emailHelp')"
    />

    <AppCheckbox
      v-if="!forgotPassword && !isCurve"
      :label="$t('account.changePassword.usePrivateKey')"
      :form="form"
      name="use_private_key"
    />

    <AppField
      v-if="!isCurve && form.values.use_private_key"
      textarea
      :rows="10"
      :form="form"
      :label="$t('account.changePassword.privateKeyLabel')"
      name="private_key"
      :placeholder="$t('account.changePassword.privateKeyPlaceholder')"
      :help="$t('account.changePassword.privateKeyHelp')"
    />

    <AppField
      v-else-if="!isCurve"
      :form="form"
      :label="$t('account.changePassword.currentPasswordLabel')"
      name="current_password"
      type="password"
      autocomplete="current-password"
      :help="$t('account.changePassword.currentPasswordHelp')"
    />

    <div class="w-1/2 sm:w-1/4">
      <AppField
        :form="form"
        :label="$t('account.changePassword.tokenLabel')"
        name="token"
        autocomplete="one-time-code"
        inputmode="numeric"
        class-add="text-sm"
        :help="$t('account.changePassword.tokenHelp')"
      />
    </div>

    <div class="border-2 p-2 pb-6 rounded-lg border-paper-300 dark:border-brownish-700">
      <AppField
        type="password"
        :form="form"
        :label="$t('account.changePassword.newPasswordLabel')"
        name="password"
        autocomplete="new-password"
        :disabled="!isCurve && !form.values.current_password && !form.values.private_key"
        :help="$t('account.changePassword.newPasswordHelp')"
      />
    </div>

    <p v-if="changePasswordError" class="text-sm text-redish-400 dark:text-redish-100">
      {{ changePasswordError }}
    </p>

    <AppButton color="info" :form="form" type="submit">{{
      $t('account.changePassword.submit')
    }}</AppButton>
  </AppForm>
</template>
