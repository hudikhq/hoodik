<script setup lang="ts">
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import { AppForm, AppField, AppButton, AppCheckbox } from '@/components/form'
import * as yup from 'yup'
import { store } from '!/auth/login'
import { store as registerStore } from '!/auth/register'
import { store as cryptoStore } from '!/crypto'
import { popIntendedRoute } from '!/auth'
import { useRouter } from 'vue-router'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ErrorResponse } from '!/api'
import type { Credentials } from 'types'
import BaseButton from '@/components/ui/BaseButton.vue'

const login = store()
const register = registerStore()
const router = useRouter()
const crypto = cryptoStore()
const { t } = useI18n()

const config = ref()
const authenticationError = ref<string | null>(null)

register.getStatus()

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
      if (typeof values.token !== 'undefined' && !values.token) {
        delete values.token
      }

      try {
        await login.withCredentials(crypto, values)
        router.push(popIntendedRoute() || { name: 'files' })
      } catch (err) {
        const error = err as ErrorResponse<unknown>
        config.value.initialErrors = error.validation || {}
        authenticationError.value = error.description
      }
    }
  }
}

init()
</script>
<template>
  <LayoutGuest>
    <SectionFullScreen v-slot="{ cardClass }" bg="pinkRed">
      <CardBox :class="cardClass">
        <h1 class="text-2xl text-white">{{ $t('auth.login.title') }}</h1>
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
            placeholder="***************************"
          />
          <div class="w-1/2 sm:w-1/4">
            <AppField
              type="password"
              :form="form"
              :label="$t('auth.login.twoFactorLabel')"
              name="token"
              autocomplete="one-time-code"
              placeholder="* * * * * *"
              class-add="text-sm"
            />
          </div>
          <AppCheckbox :label="$t('auth.rememberMe')" :form="form" name="remember" />

          <p v-if="authenticationError" class="text-sm text-redish-400">
            {{ authenticationError }}
          </p>

          <AppButton color="info" :form="form" type="submit">{{ $t('common.login') }}</AppButton>

          <BaseButton
            :to="{ name: 'login-private-key' }"
            color="light"
            :label="$t('auth.login.withPrivateKey')"
            class="float-right"
          />

          <div class="text-sm font-medium text-brownish-500 dark:text-brownish-100">
            <template v-if="register.allowRegister !== false">
              {{ $t('auth.noAccountYet') }}
              <router-link
                :to="{ name: 'register' }"
                class="text-primary-700 hover:underline dark:text-primary-500"
                >{{ $t('auth.createAnAccount') }}</router-link
              >
              <br />
            </template>
            {{ $t('auth.login.forgotPassword') }}
            <router-link
              :to="{ name: 'forgot-password' }"
              class="text-primary-700 hover:underline dark:text-primary-500"
              >{{ $t('auth.login.recoverHere') }}</router-link
            >
          </div>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
