<script setup lang="ts">
import LayoutAuthenticatedWithLoader from '@/layouts/LayoutAuthenticatedWithLoader.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import { AppForm, AppField, AppButton, AppCheckbox } from '@/components/form'
import * as yup from 'yup'
import { store } from '!/auth/login'
import { store as cryptoStore } from '!/crypto'
import { useRouter } from 'vue-router'
import { ref } from 'vue'
import * as cryptfns from '!/cryptfns'

const login = store()
const router = useRouter()
const crypto = cryptoStore()
const config = ref()

if (cryptfns.hasEncryptedPrivateKey()) {
  router.push({ name: 'decrypt' })
}

config.value = {
  initialValues: {
    password: '',
    confirm_password: '',
    logout: false
  },
  validationSchema: yup.object().shape({
    password: yup.string().required('Password is required').min(4),
    confirm_password: yup
      .string()
      .required('Please confirm your password')
      .oneOf([yup.ref('password')], 'Passwords do not match')
  }),
  onSubmit: async (values: { password: string; logout: boolean }) => {
    console.debug(values)

    const privateKey = crypto.keypair?.input

    if (!privateKey) {
      return router.push({ name: 'login' })
    }

    await cryptfns.encryptPrivateKeyAndStore(privateKey, values.password)

    if (values.logout === true) {
      login.logout(crypto)

      return router.push({ name: 'lock' })
    }

    return router.push({ name: 'home' })
  }
}
</script>
<template>
  <LayoutAuthenticatedWithLoader>
    <SectionFullScreen v-slot="{ cardClass }" bg="pinkRed">
      <CardBox :class="cardClass">
        <h1 class="text-2xl text-white mb-5">Setup Lock Screen</h1>
        <div class="flex items-start">
          <div class="flex items-center h-5">
            <p class="text-sm text-brownish-800 dark:text-brownish-500">
              Your private key will be encrypted and stored locally so the next time you come back
              you can decrypt it and access your files with a simple pin/password.
            </p>
          </div>
        </div>
        <AppForm v-if="config" :config="config" class="mt-8 space-y-6" v-slot="{ form }">
          <AppField
            type="password"
            :rows="10"
            :form="form"
            label="Set password or pin"
            name="password"
            placeholder="******"
          />
          <AppField
            type="password"
            :rows="10"
            :form="form"
            label="Confirm"
            name="confirm_password"
            placeholder="******"
          />
          <AppCheckbox label="Logout after set" :form="form" name="logout" />

          <AppButton :form="form" type="submit">Encrypt and store</AppButton>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutAuthenticatedWithLoader>
</template>
