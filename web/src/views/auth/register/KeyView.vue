<script setup lang="ts">
import { AppForm, AppField, AppButton, AppCheckbox } from '@/components/form'
import * as yup from 'yup'
import { store } from '!/auth/register'
import { useRouter } from 'vue-router'
import { ref } from 'vue'
import { rsa } from '!/cryptfns'
import LayoutGuest from '@/layouts/LayoutGuest.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import type { CreateUser } from 'types'
import * as logger from '!/logger'

const register = store()
const router = useRouter()

const config = ref()

const init = async () => {
  const initialValues = register.createUser
  const initialErrors = register.errors || {}

  if (
    !initialValues.unencrypted_private_key ||
    !initialValues.pubkey ||
    !initialValues.fingerprint
  ) {
    const kp = await rsa.generateKeyPair()
    initialValues.unencrypted_private_key = kp.input as string
    initialValues.pubkey = kp.publicKey as string
    initialValues.fingerprint = kp.fingerprint as string
  }

  config.value = {
    initialValues,
    initialErrors,
    validationSchema: yup.object().shape({
      pubkey: yup.string().required('Public key is required'),
      fingerprint: yup.string().required('Fingerprint is required'),
      unencrypted_private_key: yup.string(),
      store_private_key: yup.bool().default(true),
      i_have_stored_my_private_key: yup
        .bool()
        .default(false)
        .required('You must confirm that you have stored your private key')
        .oneOf([true], 'You must confirm that you have stored your private key')
    }),
    onSubmit: (values: Partial<CreateUser>) => {
      logger.debug(values)
      register.set(values)

      router.push({ name: 'register-two-factor' })
    }
  }
}
init()
</script>
<template>
  <LayoutGuest>
    <SectionFullScreen v-slot="{ cardClass }" bg="pinkRed">
      <CardBox :class="cardClass">
        <h1 class="text-2xl text-white">Your Private Key</h1>

        <div class="flex items-start" v-if="!config">
          <div class="flex items-center h-5">
            <p class="text-sm text-white">...is being generated, please wait...</p>
          </div>
        </div>
        <AppForm v-else :config="config" class="mt-8 space-y-6" v-slot="{ form }">
          <div class="flex items-start">
            <div class="flex items-center h-5">
              <p class="text-sm text-redish-500 dark:text-redish-400">
                <strong>This is the last time we'll show you your key!</strong> Store it somewhere
                safe. You will need this to login or recover your account.
              </p>
            </div>
          </div>
          <AppField
            textarea
            :rows="28"
            :form="form"
            label="Your private key"
            name="unencrypted_private_key"
            class-add="text-xs"
            :allow-copy="true"
          />
          <AppCheckbox
            label="I have stored my private key"
            :form="form"
            name="i_have_stored_my_private_key"
          />
          <AppButton type="submit" :disabled="!form.values.i_have_stored_my_private_key">
            Next
          </AppButton>

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
