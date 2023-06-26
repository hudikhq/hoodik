<script setup lang="ts">
import { ref } from 'vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import { AppForm, AppField } from '@/components/form'
import QuotaSlider from '@/components/ui/QuotaSlider.vue'
import * as yup from 'yup'
import type { ErrorResponse } from '!/api'
import { create } from '!/admin/invitations'
import type { Create } from 'types/admin/invitations'

const props = defineProps<{
  modelValue?: boolean | undefined
}>()

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
      email: yup.string().email().required('Email is required'),
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
      title="Invite user"
      button="info"
      buttonLabel="Invite"
      has-cancel
      @cancel="$emit('cancel')"
      :form="form"
    >
      <p v-if="errorMessage" class="text-sm text-redish-900 dark:text-redish-200">
        {{ errorMessage }}
      </p>

      <AppField :form="form" label="Email" name="email" autofocus />
      <AppField :form="form" label="Message" name="message" :textarea="true" />

      <QuotaSlider
        :model-value="form.values.quota"
        @update:model-value="(v) => form.setValues({ quota: v })"
        title="Storage quota"
      />
    </CardBoxModal>
  </AppForm>
</template>
