<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import FormError from '@/components/ui/FormError.vue'
import { AppForm, AppField } from '@/components/form'
import * as yup from 'yup'
import type { ErrorResponse } from '!/api'
import type { CryptoStore, FilesStore } from 'types'
import { humanizeError } from '!/index'

const props = defineProps<{
  modelValue?: boolean | undefined
  Storage: FilesStore
  Crypto: CryptoStore
  authenticatedUserId?: string
}>()

const emit = defineEmits(['update:modelValue', 'cancel', 'confirm'])

const { t } = useI18n()

const config = ref()
const errorMessage = ref()

const init = () => {
  config.value = {
    initialValues: {
      name: ''
    },
    validationSchema: yup.object().shape({
      name: yup.string().required(t('files.createDirectory.nameRequired'))
    }),
    onSubmit: async (values: { name: string }, ctx: any) => {
      try {
        await props.Storage.createDir(
          props.Crypto.keypair,
          values.name,
          props.Storage.dir?.id,
          props.authenticatedUserId
        )
        ctx.resetForm()
        props.Storage.find(props.Crypto.keypair, props.Storage.dir?.id || undefined)
        emit('confirm')
        emit('update:modelValue', false)
      } catch (err) {
        const error = err as ErrorResponse<unknown>
        config.value.initialErrors = error.validation || {}
        errorMessage.value = humanizeError(err)
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
      :title="$t('files.createDirectory.title')"
      button="info"
      :buttonLabel="$t('common.create')"
      has-cancel
      @cancel="$emit('cancel')"
      :form="form"
    >
      <FormError v-if="errorMessage">{{ errorMessage }}</FormError>

      <AppField
        :form="form"
        :label="$t('files.createDirectory.nameLabel')"
        name="name"
        :placeholder="$t('files.createDirectory.namePlaceholder')"
        autofocus
      />
    </CardBoxModal>
  </AppForm>
</template>
