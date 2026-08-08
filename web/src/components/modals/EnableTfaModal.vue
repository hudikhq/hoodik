<script setup lang="ts">
import { computed, ref } from 'vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import FormError from '@/components/ui/FormError.vue'
import { AppForm, AppField } from '@/components/form'
import * as yup from 'yup'
import type { ErrorResponse } from '!/api'
import { enableTwoFactor, getTwoFactorSecret } from '!/account'
import type { User } from 'types'
import QRCodeComponent from 'qrcode.vue'
import * as logger from '!/logger'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  modelValue: User
}>()

const { t } = useI18n()
const emit = defineEmits(['update:modelValue', 'cancel', 'confirm'])

const user = computed({
  get: () => props.modelValue,
  set: (v) => emit('update:modelValue', v)
})

const config = ref()
const errorMessage = ref()

type Values = {
  token: string
  secret: string
}

const secret = ref('')
const qrcode = computed(() => {
  const issuer = 'Hoodik Encrypted File Storage'

  return `otpauth://totp/${issuer}:${user.value.email}?secret=${secret.value}&issuer=${issuer}`
})

const init = async () => {
  secret.value = (await getTwoFactorSecret()) as string

  config.value = {
    initialValues: {
      token: '',
      secret: secret.value
    } as Values,
    validationSchema: yup.object().shape({
      secret: yup.string().required(t('account.tfa.secretRequired')),
      token: yup.string().required(t('account.tfa.tokenRequired'))
    }),
    onSubmit: async (values: Values, ctx: any) => {
      logger.debug(values)
      try {
        await enableTwoFactor(values.secret, values.token)
        user.value.secret = true
        ctx.resetForm()
        emit('confirm')
      } catch (err) {
        const error = err as ErrorResponse<unknown>
        ctx.setErrors(error.validation || {})
        errorMessage.value = error.description || error.message
      }
    }
  }
}

init()
</script>

<template>
  <AppForm v-if="config" :config="config" v-slot="{ form }">
    <CardBoxModal
      :modelValue="true"
      :title="$t('account.tfa.enableTitle')"
      button="info"
      :buttonLabel="$t('account.tfa.enable')"
      has-cancel
      @cancel="$emit('cancel')"
      :form="form"
    >
      <div class="flex items-start mb-2">
        <div class="flex items-center h-5">
          <p class="text-sm dark:text-white">
            {{ $t('account.tfa.scanQr') }}
          </p>
        </div>
      </div>

      <div class="mb-4">
        <Suspense>
          <QRCodeComponent
            :value="qrcode"
            :size="200"
            render-as="svg"
            :margin="2"
            level="H"
            class="center-self"
          />
        </Suspense>
      </div>

      <AppField
        :form="form"
        :label="$t('account.tfa.secretLabel')"
        name="secret"
        :allow-copy="true"
      />
      <AppField :form="form" :label="$t('account.tfa.tokenLabel')" name="token" />

      <FormError v-if="errorMessage">{{ errorMessage }}</FormError>
    </CardBoxModal>
  </AppForm>
</template>
