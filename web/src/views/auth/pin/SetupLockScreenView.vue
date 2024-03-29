<script setup lang="ts">
import LayoutAuthenticatedWithLoader from '@/layouts/LayoutAuthenticatedWithLoader.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import { AppForm, AppField, AppButton } from '@/components/form'
import * as yup from 'yup'
import { pk } from '!/auth'
import { store as loginStore } from '!/auth/login'
import { store as cryptoStore } from '!/crypto'
import { useRouter } from 'vue-router'
import { ref } from 'vue'
import * as logger from '!/logger'

const login = loginStore()
const router = useRouter()
const crypto = cryptoStore()
const config = ref()

if (pk.hasPin()) {
  router.push({ name: 'decrypt', replace: true })
}

config.value = {
  initialValues: {
    password: '',
    confirm_password: ''
  },
  validationSchema: yup.object().shape({
    password: yup.string().required('Password is required').min(4),
    confirm_password: yup
      .string()
      .required('Please confirm your password')
      .oneOf([yup.ref('password')], 'Passwords do not match')
  }),
  onSubmit: async (values: { password: string; logout: boolean }) => {
    logger.debug(values)

    const privateKey = crypto.keypair?.input

    if (!privateKey) {
      return router.push({ name: 'login' })
    }

    await pk.pinEncryptAndStore(
      privateKey,
      values.password,
      login.authenticated?.user?.email as string
    )

    return router.push({ name: 'files', replace: true })
  }
}
</script>
<template>
  <LayoutAuthenticatedWithLoader clear>
    <SectionFullScreen v-slot="{ cardClass }" bg="pinkRed">
      <CardBox :class="cardClass">
        <h1 class="text-2xl text-white mb-5">Setup Lock Screen</h1>
        <div class="flex items-start">
          <div class="flex items-center">
            <p class="text-sm">
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

          <AppButton :form="form" type="submit">Encrypt and store</AppButton>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutAuthenticatedWithLoader>
</template>
