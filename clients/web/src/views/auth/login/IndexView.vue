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

const login = store()
const router = useRouter()
const crypto = cryptoStore()

const config = ref()
const authenticationError = ref<string | null>(null)

const init = () => {
  config.value = {
    initialValues: {
      remember: false
    },
    validationSchema: yup.object().shape({
      email: yup.string().required('Email is required').email('Email is invalid'),
      password: yup.string().required('Password is required')
    }),
    onSubmit: async (values: Credentials) => {
      console.debug(values)

      try {
        await login.withCredentials(crypto, values)
        router.push('/')
      } catch (err) {
        console.error(err)
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
        <h1 class="text-2xl text-white">Access Your Files</h1>
        <AppForm v-if="config" :config="config" class="mt-8 space-y-6" v-slot="{ form }">
          <AppField
            :form="form"
            label="Your email"
            name="email"
            placeholder="your@email.com"
            autofocus
          />
          <AppField
            type="password"
            :form="form"
            label="Your password"
            name="password"
            placeholder="***************************"
          />
          <div class="w-1/4">
            <AppField
              type="password"
              :form="form"
              label="2FA token (optional)"
              name="token"
              placeholder="* * * * * *"
              class-add="text-sm"
            />
          </div>
          <AppCheckbox label="Remember me" :form="form" name="remember" />

          <p v-if="authenticationError" class="text-sm text-red-400">
            {{ authenticationError }}
          </p>

          <AppButton :form="form" type="submit">Login</AppButton>

          <router-link
            to="/auth/login/private-key"
            class="float-right rounded-md text-green-200 py-2 px-4 border border-green-300"
          >
            Login With Private Key
          </router-link>

          <div class="text-sm font-medium text-gray-500 dark:text-gray-400">
            Don't have an account yet?
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
