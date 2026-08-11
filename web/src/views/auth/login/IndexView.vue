<script setup lang="ts">
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import { AppForm, AppField, AppButton, AppCheckbox, AppCodeField } from '@/components/form'
import * as yup from 'yup'
import { store } from '!/auth/login'
import { store as registerStore } from '!/auth/register'
import { store as cryptoStore } from '!/crypto'
import { popIntendedRoute } from '!/auth'
import { useRouter } from 'vue-router'
import { onUnmounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ErrorResponse } from '!/api'
import type { Credentials } from 'types'
import BaseButton from '@/components/ui/BaseButton.vue'
import { humanizeError } from '!/index'

const login = store()
const register = registerStore()
const router = useRouter()
const crypto = cryptoStore()
const { t } = useI18n()

const config = ref()
const tokenConfig = ref()
const codeField = ref<{ clear: () => void } | null>(null)
const authenticationError = ref<string | null>(null)

register.getStatus()

/**
 * The server proves the password before it looks at the code, and answers
 * `two_factor_required` only once it has. So this is the signal that the
 * password was right and the account wants its second factor — and because the
 * password held, the server does not charge it against the login throttle.
 * A wrong code later comes back as `invalid_otp_token` instead.
 */
const isTfaPrompt = (err: unknown) =>
  (err as ErrorResponse<unknown>)?.body?.message === 'two_factor_required'

const isBadCode = (err: unknown) =>
  (err as ErrorResponse<unknown>)?.body?.message === 'invalid_otp_token'

/**
 * Held only between the two steps. The OPAQUE login state is single-use and
 * consumed by the failed attempt, so the second step has to run the exchange
 * again from the start — which means keeping the password until it succeeds.
 */
const pending = ref<Credentials | null>(null)

const forget = () => (pending.value = null)

onUnmounted(forget)

const finish = () => {
  forget()
  router.push(popIntendedRoute() || { name: 'files' })
}

const init = () => {
  config.value = {
    initialValues: {
      remember: false
    },
    validationSchema: yup.object().shape({
      email: yup
        .string()
        .required(t('auth.validation.emailRequired'))
        .email(t('auth.validation.emailInvalid')),
      password: yup.string().required(t('auth.validation.passwordRequired'))
    }),
    onSubmit: async (values: Credentials) => {
      authenticationError.value = null

      try {
        await login.withCredentials(crypto, values)
        finish()
      } catch (err) {
        if (isTfaPrompt(err)) {
          pending.value = { ...values }
          initToken()
          return
        }

        const error = err as ErrorResponse<unknown>
        config.value.initialErrors = error.validation || {}
        authenticationError.value = humanizeError(err)
      }
    }
  }
}

const initToken = () => {
  tokenConfig.value = {
    initialValues: { token: '' },
    validationSchema: yup.object().shape({
      token: yup.string().required(t('auth.validation.tokenRequired'))
    }),
    onSubmit: async ({ token }: { token: string }) => {
      if (!pending.value) return
      authenticationError.value = null

      try {
        await login.withCredentials(crypto, { ...pending.value, token })
        finish()
      } catch (err) {
        authenticationError.value = humanizeError(err)
        codeField.value?.clear()

        // Anything other than a bad code means the credentials themselves no
        // longer work — a re-typed code cannot fix that, so start over.
        if (!isBadCode(err)) {
          forget()
          tokenConfig.value = undefined
        }
      }
    }
  }
}

const back = () => {
  forget()
  tokenConfig.value = undefined
  authenticationError.value = null
}

init()
</script>
<template>
  <LayoutGuest>
    <SectionFullScreen v-slot="{ cardClass }">
      <CardBox :class="cardClass">
        <template v-if="pending && tokenConfig">
          <h1 class="text-2xl text-brownish-700 dark:text-white">
            {{ $t('auth.login.twoFactorTitle') }}
          </h1>
          <p class="mt-2 text-sm text-brownish-400 dark:text-brownish-50">
            {{ $t('auth.login.twoFactorBody') }}
          </p>

          <AppForm :config="tokenConfig" class="mt-8 space-y-6" v-slot="{ form, submit }">
            <AppCodeField
              ref="codeField"
              :form="form"
              :label="$t('auth.login.twoFactorLabel')"
              name="token"
              data-testid="login-two-factor"
              autofocus
              @complete="() => submit()"
            />

            <p v-if="authenticationError" class="text-sm text-redish-700 dark:text-redish-100">
              {{ authenticationError }}
            </p>

            <AppButton color="info" :form="form" type="submit">{{ $t('common.login') }}</AppButton>

            <BaseButton
              color="light"
              :label="$t('common.back')"
              class="float-right"
              @click="back"
            />
          </AppForm>
        </template>

        <template v-else>
          <h1 class="text-2xl text-brownish-700 dark:text-white">{{ $t('auth.login.title') }}</h1>
          <AppForm v-if="config" :config="config" class="mt-8 space-y-6" v-slot="{ form }">
            <AppField
              :form="form"
              :label="$t('auth.yourEmail')"
              name="email"
              :placeholder="$t('auth.emailPlaceholder')"
              autocomplete="username"
              autofocus
            />
            <AppField
              type="password"
              :form="form"
              :label="$t('auth.yourPassword')"
              name="password"
              autocomplete="current-password"
              placeholder="••••••••"
            />
            <AppCheckbox :label="$t('auth.rememberMe')" :form="form" name="remember" />

            <p v-if="authenticationError" class="text-sm text-redish-700 dark:text-redish-100">
              {{ authenticationError }}
            </p>

            <AppButton color="info" :form="form" type="submit">{{ $t('common.login') }}</AppButton>

            <BaseButton
              :to="{ name: 'login-private-key' }"
              color="light"
              :label="$t('auth.login.withPrivateKey')"
              class="float-right"
            />

            <div class="text-sm font-medium text-brownish-500 dark:text-brownish-50">
              <template v-if="register.allowRegister !== false">
                {{ $t('auth.noAccountYet') }}
                <router-link
                  :to="{ name: 'register' }"
                  class="text-primary-700 hover:underline dark:text-primary-100"
                  >{{ $t('auth.createAnAccount') }}</router-link
                >
                <br />
              </template>
              {{ $t('auth.login.forgotPassword') }}
              <router-link
                :to="{ name: 'forgot-password' }"
                class="text-primary-700 hover:underline dark:text-primary-100"
                >{{ $t('auth.login.recoverHere') }}</router-link
              >
            </div>
          </AppForm>
        </template>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
