<script setup lang="ts">
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import { AppForm, AppField, AppButton, AppCheckbox } from '@/components/form'
import * as yup from 'yup'
import { store } from '!/auth/login'
import { store as registerStore } from '!/auth/register'
import { parseBundle } from '!/auth/bundle'
import { store as cryptoStore } from '!/crypto'
import { popIntendedRoute } from '!/auth'
import { useRouter } from 'vue-router'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ErrorResponse } from '!/api'
import * as cryptfns from '!/cryptfns'
import type { PrivateKeyLogin } from 'types'
import * as logger from '!/logger'
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
    initialValues: { remember: false },
    validationSchema: yup.object().shape({
      privateKey: yup
        .string()
        .required(t('auth.validation.privateKeyRequired'))
        .test({
          name: 'privateKey',
          message: t('auth.validation.privateKeyInvalid'),
          test: async (value) => {
            // A v2 account pastes its recovery bundle (`v1|ed:|x:`); a legacy
            // account pastes its RSA PEM. Accept either shape.
            if (value && value.includes('ed:') && value.includes('x:')) {
              const { identity, wrapping } = parseBundle(value)
              return Boolean(identity && wrapping)
            }
            try {
              await cryptfns.rsa.inputToKeyPair(value)
              return true
            } catch (err) {
              return false
            }
          }
        })
    }),
    onSubmit: async (values: PrivateKeyLogin) => {
      logger.debug(values)

      try {
        await login.withPrivateKey(crypto, values)
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
    <SectionFullScreen v-slot="{ cardClass }">
      <CardBox :class="cardClass">
        <h1 class="text-2xl text-brownish-700 dark:text-white">{{ $t('auth.login.title') }}</h1>
        <AppForm v-if="config" :config="config" class="mt-8 space-y-6" v-slot="{ form }">
          <AppField
            textarea
            :rows="10"
            :form="form"
            :label="$t('auth.privateKey.label')"
            name="privateKey"
            :placeholder="$t('auth.privateKey.placeholder')"
            :help="$t('auth.privateKey.help')"
          />
          <AppCheckbox :label="$t('auth.rememberMe')" :form="form" name="remember" />

          <p v-if="authenticationError" class="text-sm text-redish-400 dark:text-redish-100">
            {{ authenticationError }}
          </p>

          <AppButton color="info" :form="form" type="submit">{{ $t('common.login') }}</AppButton>

          <BaseButton
            :to="{ name: 'login' }"
            color="light"
            :label="$t('auth.privateKey.withCredentials')"
            class="float-right"
          />

          <div
            v-if="register.allowRegister !== false"
            class="text-sm font-medium text-brownish-500 dark:text-brownish-50"
          >
            {{ $t('auth.noAccountYet') }}
            <router-link
              :to="{ name: 'register' }"
              class="text-primary-700 hover:underline dark:text-primary-100"
              >{{ $t('auth.createAnAccount') }}</router-link
            >
          </div>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
