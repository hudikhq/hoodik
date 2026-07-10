<script setup lang="ts">
import { AppForm, AppField, AppButton, AppCheckbox } from '@/components/form'
import * as yup from 'yup'
import { store } from '!/auth/register'
import { encodeBundle } from '!/auth/bundle'
import { useRouter } from 'vue-router'
import { ref } from 'vue'
import { ed25519, wrapping } from '!/cryptfns'
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
    !initialValues.identity_private_key ||
    !initialValues.pubkey ||
    !initialValues.wrapping_private_key ||
    !initialValues.fingerprint
  ) {
    const edPriv = await ed25519.generatePrivateKey()
    const edPub = await ed25519.publicFromPrivate(edPriv)
    const xPriv = await wrapping.generatePrivateKey()
    const xPub = await wrapping.publicFromPrivate(xPriv)

    initialValues.identity_private_key = edPriv
    initialValues.pubkey = edPub
    initialValues.wrapping_private_key = xPriv
    initialValues.wrapping_pubkey = xPub
    initialValues.fingerprint = await ed25519.fingerprint(edPub)
  }

  // The recovery bundle is the exact material sealed into the account's
  // envelope; a user who backs it up can log in via "private key" and recover
  // access even without the password.
  initialValues.recovery_bundle = encodeBundle({
    identity: initialValues.identity_private_key,
    wrapping: initialValues.wrapping_private_key
  })

  config.value = {
    initialValues,
    initialErrors,
    validationSchema: yup.object().shape({
      pubkey: yup.string().required('Public key is required'),
      fingerprint: yup.string().required('Fingerprint is required'),
      recovery_bundle: yup.string(),
      store_private_key: yup.bool().default(true),
      i_have_stored_my_private_key: yup
        .bool()
        .default(false)
        .required('You must confirm that you have stored your recovery key')
        .oneOf([true], 'You must confirm that you have stored your recovery key')
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
        <h1 class="text-2xl text-white">Your Recovery Key</h1>

        <div class="flex items-start" v-if="!config">
          <div class="flex items-center h-5">
            <p class="text-sm text-white">...is being generated, please wait...</p>
          </div>
        </div>
        <AppForm v-else :config="config" class="mt-8 space-y-6" v-slot="{ form }">
          <div class="flex items-start">
            <div class="flex items-center h-5">
              <p class="text-sm text-redish-500 dark:text-redish-400">
                <strong>This is the last time we'll show you your recovery key!</strong> Store it
                somewhere safe. You will need this to login or recover your account if you forget
                your password.
              </p>
            </div>
          </div>
          <AppField
            textarea
            :rows="12"
            :form="form"
            label="Your recovery key"
            name="recovery_bundle"
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

          <div class="text-sm font-medium text-brownish-500 dark:text-brownish-100">
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
