<script setup lang="ts">
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import { AppForm, AppField, AppButton } from '@/components/form'
import * as yup from 'yup'
import { store as loginStore } from '!/auth/login'
import { store as cryptoStore } from '!/crypto'
import { pk, popIntendedRoute } from '!/auth'
import { useRouter } from 'vue-router'
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import type { ErrorResponse } from '!/api'
import type { Credentials } from 'types'

const login = loginStore()
const router = useRouter()
const crypto = cryptoStore()
const { t } = useI18n()

const config = ref()
const authenticationError = ref<string | null>(null)

/**
 * Email of the account that has stored private key with pin
 */
const email = computed(() => {
  const e = pk.getPinEmail()

  return e || undefined
})

/**
 * Forget the stored private key and redirect to login page
 */
const forget = async () => {
  pk.clearPin()
}

const init = () => {
  if (!pk.hasPin()) {
    return router.push({ name: 'login', replace: true })
  }

  config.value = {
    initialValues: {
      password: ''
    },
    validationSchema: yup.object().shape({
      password: yup.string().required(t('auth.validation.passwordRequired'))
    }),
    onSubmit: async (values: Credentials) => {
      try {
        await login.withPin(crypto, values.password)

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
      <CardBox :class="cardClass" v-if="config">
        <h1 class="text-2xl text-brownish-700 dark:text-white mb-5">{{ $t('auth.decrypt.title') }}</h1>
        <p>
          {{ $t('auth.decrypt.aboutToUnlock') }} <strong>{{ email }}</strong>
          {{ $t('auth.decrypt.notYou') }}
          <router-link :to="{ name: 'login' }" @click="forget" class="regular-link">{{
            $t('auth.decrypt.loginLink')
          }}</router-link>
        </p>

        <AppForm :config="config" class="mt-8 space-y-6" v-slot="{ form }">
          <AppField
            type="password"
            :form="form"
            :label="$t('auth.yourPassword')"
            name="password"
            autocomplete="off"
            placeholder="••••••••"
            :autofocus="true"
          />

          <p v-if="authenticationError" class="text-sm text-redish-400">
            {{ authenticationError }}
          </p>

          <AppButton color="info" :form="form" type="submit">{{ $t('auth.unlock') }}</AppButton>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
