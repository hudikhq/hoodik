<script setup lang="ts">
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import { AppForm, AppField, AppButton } from '@/components/form'
import * as yup from 'yup'
import { store as loginStore } from '!/auth/login'
import { store as cryptoStore } from '!/crypto'
import { pk } from '!/auth'
import { useRouter } from 'vue-router'
import { computed, ref } from 'vue'
import type { ErrorResponse } from '!/api'
import type { Credentials } from 'types'

const login = loginStore()
const router = useRouter()
const crypto = cryptoStore()

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
      password: yup.string().required('Password is required')
    }),
    onSubmit: async (values: Credentials) => {
      try {
        await login.withPin(crypto, values.password)

        router.push({ name: 'files' })
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
        <h1 class="text-2xl text-white mb-5">Unlock Your Account</h1>
        <p>
          You are about to unlock an account of <strong>{{ email }}</strong> if this isn't you,
          please go to
          <router-link :to="{ name: 'login' }" @click="forget" class="regular-link"
            >login.</router-link
          >
        </p>

        <AppForm :config="config" class="mt-8 space-y-6" v-slot="{ form }">
          <AppField
            type="password"
            :form="form"
            label="Your password"
            name="password"
            placeholder="********"
            :autofocus="true"
          />

          <p v-if="authenticationError" class="text-sm text-redish-400">
            {{ authenticationError }}
          </p>

          <AppButton color="info" :form="form" type="submit">Unlock</AppButton>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
