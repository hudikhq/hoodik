<script setup lang="ts">
import { ref } from 'vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import FormError from '@/components/ui/FormError.vue'
import { AppForm, AppField } from '@/components/form'
import QuotaSlider from '@/components/ui/QuotaSlider.vue'
import * as yup from 'yup'
import type { ErrorResponse } from '!/api'
import { create } from '!/admin/invitations'
import type { Create } from 'types/admin/invitations'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  modelValue?: boolean | undefined
}>()

const { t } = useI18n()

const emit = defineEmits(['update:modelValue', 'cancel', 'confirm'])

const config = ref()
const errorMessage = ref()

const init = () => {
  config.value = {
    initialValues: {
      email: '',
      message: '',
      quota: undefined,
      role: undefined
    } as Create,
    validationSchema: yup.object().shape({
      email: yup.string().email().required(t('account.invite.emailRequired')),
      quota: yup.number().min(0)
    }),
    onSubmit: async (values: Create, ctx: any) => {
      try {
        await create(values)
        ctx.resetForm()
        emit('confirm')
        emit('update:modelValue', false)
      } catch (err) {
        const error = err as ErrorResponse<unknown>
        config.value.initialErrors = error.validation || {}
        errorMessage.value = error.description
      }
    }
  }
}

init()
</script>

<template>
  <AppForm v-if="config" :config="config" v-slot="{ form }">
    <CardBoxModal
      :modelValue="props.modelValue"
      @update:modelValue="$emit('update:modelValue', $event)"
      :title="$t('account.invite.title')"
      button="info"
      :buttonLabel="$t('account.invite.submit')"
      has-cancel
      @cancel="$emit('cancel')"
      :form="form"
    >
      <FormError v-if="errorMessage">{{ errorMessage }}</FormError>

      <AppField :form="form" :label="$t('common.email')" name="email" autofocus />
      <AppField
        :form="form"
        :label="$t('account.invite.messageLabel')"
        name="message"
        :textarea="true"
      />

      <QuotaSlider
        :model-value="form.values.quota"
        @update:model-value="(v) => form.setValues({ quota: v })"
        :title="$t('account.invite.quotaTitle')"
      />
    </CardBoxModal>
  </AppForm>
</template>
