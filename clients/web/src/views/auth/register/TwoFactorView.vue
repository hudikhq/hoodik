<script setup lang="ts">
import { AppForm, AppField, AppButton } from '@/components/form'
import * as yup from 'yup'
import { store } from '@/stores/auth/register'
import type { CreateUser } from '@/stores/auth/register'
import { useRouter } from 'vue-router'
import { computed, ref } from 'vue'
import type { ErrorResponse } from '@/stores/api'
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import QRCodeComponent from 'qrcode.vue'

const register = store()
const router = useRouter()

const config = ref()

const secret = ref<string>()
const email = ref<string>()

const qrcode = computed(() => {
  const issuer = 'Hoodik Encrypted File Storage'

  return `otpauth://totp/${issuer}:${email.value}?secret=${secret.value}&issuer=${issuer}`
})

const init = async () => {
  const initialValues = register.createUser
  const initialErrors = register.errors || {}
  secret.value = (await register.getTwoFactorSecret()) as string
  email.value = register.createUser.email

  config.value = {
    initialValues: {
      ...initialValues,
      secret: secret.value,
      token: ''
    },
    initialErrors,
    validationSchema: yup.object().shape({
      secret: yup.string().required('Secret is required'),
      token: yup.string().required('Two factor token is required')
    }),
    onSubmit: async (values: Partial<CreateUser>) => {
      console.log(values)
      register.set(values)

      try {
        const response = await register.register(register.createUser)
        console.debug(response)
        register.clear()
        router.push('/')
      } catch (err) {
        const error = err as ErrorResponse<unknown>
        register.setErrors(error.validation)
        console.log(register.createUser)

        if (error?.validation?.email || error?.validation?.password) {
          router.push('/auth/register')
        } else if (error?.validation?.pubkey || error?.validation?.fingerprint) {
          router.push('/auth/register/key')
        } else if (error?.validation?.token) {
          config.value.initialErrors = register.errors
        } else {
          throw err
        }
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
        <h1 class="text-2xl text-white">Two Factor Authentication</h1>

        <div class="flex items-start" v-if="!config">
          <div class="flex items-center h-5">
            <p class="text-sm text-white">We are generating your two factor secret...</p>
          </div>
        </div>

        <AppForm v-else :config="config" class="mt-8 space-y-6" v-slot="{ form }">
          <div class="flex items-start">
            <div class="flex items-center h-5">
              <p class="text-sm dark:text-white">
                Scan the QR code with your two factor application, or simply copy and paste the
                secret code below
              </p>
            </div>
          </div>

          <Suspense>
            <template #fallback> Loading... </template>
            <QRCodeComponent
              :value="qrcode"
              :size="200"
              render-as="svg"
              :margin="2"
              level="H"
              class="center-self"
            />
          </Suspense>

          <AppField :form="form" label="Your two factor secret" name="secret" :allow-copy="true" />
          <AppField :rows="28" :form="form" label="Enter your two factor token" name="token" />

          <AppButton type="submit"> Register with Two Factor </AppButton>

          <AppButton
            type="button"
            @click="() => config.onSubmit(form.values)"
            class="float-right rounded-md text-green-200 py-2 px-4 border border-green-300"
          >
            Skip and Register
          </AppButton>

          <div class="text-sm font-medium text-gray-500 dark:text-gray-400">
            Already have an account?
            <router-link
              to="/auth/login"
              class="text-primary-700 hover:underline dark:text-primary-500"
              >Login</router-link
            >
          </div>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutGuest>
</template>

<style scope lang="css">
.center-self {
  margin: auto;
}
</style>
