<script setup lang="ts">
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import { AppForm, AppField, AppButton } from '@/components/form'
import * as yup from 'yup'
import { store } from '!/auth/login'
import { store as cryptoStore } from '!/crypto'
import { useRouter } from 'vue-router'
import { ref } from 'vue'
import type { ErrorResponse } from '!/api'
import * as cryptfns from '!/cryptfns'
import type { PrivateKeyLogin } from 'types'
import * as logger from '!/logger'
import BaseButton from '@/components/ui/BaseButton.vue'

const login = store()
const router = useRouter()
const crypto = cryptoStore()

const config = ref()
const authenticationError = ref<string | null>(null)

const init = () => {
  config.value = {
    initialValues: {
      privateKey: '',
      remember: false
    },
    validationSchema: yup.object().shape({
      privateKey: yup
        .string()
        .required('Private key is required')
        .test({
          name: 'privateKey',
          message: 'Invalid private key',
          test: async (value) => {
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
        router.push({ name: 'home' })
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
        <h1 class="text-2xl text-white">Access Your Files</h1>
        <AppForm v-if="config" :config="config" class="mt-8 space-y-6" v-slot="{ form }">
          <AppField
            textarea
            :rows="10"
            :form="form"
            label="Your private key"
            name="privateKey"
            placeholder="Paste your private key here"
            help="Your private key will never be sent to the server, we will only use it to generate fingerprint and sign your requests to try and authenticate you"
          />

          <p v-if="authenticationError" class="text-sm text-redish-400">
            {{ authenticationError }}
          </p>

          <AppButton color="info" :form="form" type="submit">Login</AppButton>

          <BaseButton
            :to="{ name: 'login' }"
            color="light"
            label="Login With Email and Password"
            class="float-right"
          />

          <div class="text-sm font-medium text-brownish-500 dark:text-brownish-400">
            Don't have an account yet?
            <router-link
              :to="{ name: 'register' }"
              class="text-primary-700 hover:underline dark:text-primary-500"
              >Create an Account</router-link
            >
          </div>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
