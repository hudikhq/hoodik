<script setup lang="ts">
import LayoutAuthenticatedWithLoader from '@/layouts/LayoutAuthenticatedWithLoader.vue'
import SectionFullScreen from '@/components/ui/SectionFullScreen.vue'
import CardBox from '@/components/ui/CardBox.vue'
import { AppForm, AppField, AppButton } from '@/components/form'
import * as yup from 'yup'
import { pk } from '!/auth'
import { recoveryKeyFor } from '!/auth/bundle'
import { store as loginStore } from '!/auth/login'
import { store as cryptoStore } from '!/crypto'
import { useRouter } from 'vue-router'
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import * as logger from '!/logger'

const login = loginStore()
const router = useRouter()
const crypto = cryptoStore()
const { t } = useI18n()
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
    password: yup.string().required(t('auth.validation.passwordRequired')).min(4),
    confirm_password: yup
      .string()
      .required(t('auth.validation.confirmPasswordRequired'))
      .oneOf([yup.ref('password')], t('auth.validation.passwordsDoNotMatch'))
  }),
  onSubmit: async (values: { password: string; logout: boolean }) => {
    logger.debug(values)

    const material = crypto.keypair ? recoveryKeyFor(crypto.keypair) : ''

    if (!material) {
      return router.push({ name: 'login' })
    }

    await pk.pinEncryptAndStore(
      material,
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
        <h1 class="text-2xl text-white mb-5">{{ $t('auth.lockSetup.title') }}</h1>
        <div class="flex items-start">
          <div class="flex items-center">
            <p class="text-sm">
              {{ $t('auth.lockSetup.description') }}
            </p>
          </div>
        </div>
        <AppForm v-if="config" :config="config" class="mt-8 space-y-6" v-slot="{ form }">
          <AppField
            type="password"
            :rows="10"
            :form="form"
            :label="$t('auth.lockSetup.passwordLabel')"
            name="password"
            autocomplete="off"
            placeholder="******"
          />
          <AppField
            type="password"
            :rows="10"
            :form="form"
            :label="$t('common.confirm')"
            name="confirm_password"
            autocomplete="off"
            placeholder="******"
          />

          <AppButton :form="form" type="submit">{{ $t('auth.lockSetup.submit') }}</AppButton>
        </AppForm>
      </CardBox>
    </SectionFullScreen>
  </LayoutAuthenticatedWithLoader>
</template>
