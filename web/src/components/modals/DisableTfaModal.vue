<script setup lang="ts">
import { computed, ref } from 'vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import { AppForm, AppField } from '@/components/form'
import * as yup from 'yup'
import type { ErrorResponse } from '!/api'
import { disableTwoFactor } from '!/account'
import type { User } from 'types'
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
}

const init = async () => {
  config.value = {
    initialValues: {
      token: ''
    } as Values,
    validationSchema: yup.object().shape({
      token: yup.string().required(t('account.tfa.tokenRequired'))
    }),
    onSubmit: async (values: Values, ctx: any) => {
      logger.debug(values)
      try {
        await disableTwoFactor(values.token)
        user.value.secret = false
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
      :title="$t('account.tfa.disableTitle')"
      button="danger"
      :buttonLabel="$t('account.tfa.disable')"
      has-cancel
      @cancel="$emit('cancel')"
      :form="form"
    >
      <AppField
        :form="form"
        :label="$t('account.tfa.tokenLabel')"
        name="token"
        :help="$t('account.tfa.lostAccessHelp')"
      />

      <p v-if="errorMessage" class="text-sm text-redish-900 dark:text-redish-200">
        {{ errorMessage }}
      </p>
    </CardBoxModal>
  </AppForm>
</template>
