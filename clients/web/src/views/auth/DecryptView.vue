<script setup lang="ts">
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import { AppForm, AppField, AppButton, AppCheckbox } from '@/components/form'
import * as yup from 'yup'
import { store, type Credentials } from '@/stores/auth/login'
import { store as cryptoStore } from '@/stores/crypto'
import { useRouter } from 'vue-router'
import { ref } from 'vue'
import type { ErrorResponse } from '@/stores/api'
import * as cryptfns from '@/stores/cryptfns'

const login = store()
const router = useRouter()
const crypto = cryptoStore()

const hasAuth = ref(true)

const config = ref()
const authenticationError = ref<string | null>(null)

const init = () => {
  if (!cryptfns.hasEncryptedPrivateKey()) {
    hasAuth.value = false
    return setTimeout(() => {
      router.push({ path: '/auth/login', replace: true })
    }, 5000)
  }

  config.value = {
    initialValues: {
      password: ''
    },
    validationSchema: yup.object().shape({
      password: yup.string().required('Password is required')
    }),
    onSubmit: async (values: Credentials) => {
      console.debug(values)

      try {
        await login.withPin(crypto, values.password)

        router.push('/')
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
      <CardBox :class="`${cardClass} text-center`" v-if="!config">
        <div class="flex items-start">
          <div class="flex items-center h-5">
            <p class="text-sm">
              You don't have lock screen setup yet. You will be redirected to the login page in
              couple of seconds to authenticate and then you can setup your lock screen.
            </p>
          </div>
        </div>

        <router-link
          to="/auth/login"
          class="float-right rounded-md text-red-200 py-2 px-4 border border-red-300"
        >
          Login
        </router-link>
      </CardBox>

      <CardBox :class="cardClass" v-else>
        <h1 class="text-2xl text-white mb-5">Unlock Your Account</h1>

        <AppForm :config="config" class="mt-8 space-y-6" v-slot="{ form }">
          <AppField
            type="password"
            :form="form"
            label="Your password"
            name="password"
            placeholder="********"
            :autofocus="true"
          />
          <AppCheckbox label="Remember me" :form="form" name="remember" />

          <p v-if="authenticationError" class="text-sm text-red-400">
            {{ authenticationError }}
          </p>

          <AppButton :form="form" type="submit">Unlock</AppButton>

          <div class="text-sm font-medium text-gray-500 dark:text-gray-400">
            Not your account?
            <router-link
              to="/auth/register"
              class="text-primary-700 hover:underline dark:text-primary-500"
              >Create an Account</router-link
            >
          </div>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
