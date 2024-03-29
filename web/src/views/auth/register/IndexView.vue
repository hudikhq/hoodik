<script setup lang="ts">
import { AppForm, AppField, AppButton } from '@/components/form'
import * as yup from 'yup'
import { zxcvbn } from '@zxcvbn-ts/core'
import { store } from '!/auth/register'
import { useRoute, useRouter } from 'vue-router'
import { ref } from 'vue'
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import type { CreateUser } from 'types'
import * as logger from '!/logger'

const register = store()
const router = useRouter()
const route = useRoute()

const config = ref()
const working = ref(false)

const init = () => {
  register.preload(route)

  const initialValues = register.createUser
  const initialErrors = register.errors || {}

  config.value = {
    initialValues: initialValues,
    initialErrors,
    validationSchema: yup.object().shape({
      email: yup.string().required('Email is required').email('Email is invalid'),
      password: yup
        .string()
        .required('Password is required')
        .test(
          'weak-password',
          'Password used is too weak',
          (value: string) => zxcvbn(value).score >= 3
        ),
      confirm_password: yup
        .string()
        .required('Please confirm your password')
        .oneOf([yup.ref('password')], 'Passwords do not match')
    }),
    onSubmit: async (values: Partial<CreateUser>) => {
      logger.debug(values)
      register.set(values)

      router.push({ name: 'register-key' })
      working.value = true
    }
  }
}

init()
</script>
<template>
  <LayoutGuest>
    <SectionFullScreen v-slot="{ cardClass }" bg="pinkRed">
      <CardBox :class="cardClass">
        <h1 class="text-2xl text-white">Create Account</h1>
        <AppForm
          v-if="config"
          :config="config"
          :working="working"
          class="mt-8 space-y-6"
          v-slot="{ form }"
        >
          <AppField
            :form="form"
            label="Your email"
            name="email"
            placeholder="your@email.com"
            :disabled="form.values.invitation_id"
          />
          <AppField
            type="password"
            :form="form"
            label="Your password"
            name="password"
            placeholder="*********"
          />
          <AppField
            :allow-copy="false"
            type="password"
            :form="form"
            label="Confirm your password"
            name="confirm_password"
            placeholder="*********"
          />
          <AppButton color="info" type="submit">Next</AppButton>

          <div class="text-sm font-medium text-brownish-500 dark:text-brownish-400">
            Already have an account?
            <router-link
              :to="{ name: 'login' }"
              class="text-primary-700 hover:underline dark:text-primary-500"
              >Login</router-link
            >
          </div>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>
