<script setup lang="ts">
import { ref } from 'vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import { AppForm, AppField } from '@/components/form'
import * as yup from 'yup'
import type { ErrorResponse } from '!/api'
import type { CryptoStore, FilesStore } from 'types'

const props = defineProps<{
  modelValue?: boolean | undefined
  Storage: FilesStore
  Crypto: CryptoStore
}>()

const emit = defineEmits(['update:modelValue', 'cancel', 'confirm'])

const config = ref()
const errorMessage = ref()

const init = () => {
  config.value = {
    initialValues: {
      name: ''
    },
    validationSchema: yup.object().shape({
      name: yup.string().required('Directory name is required')
    }),
    onSubmit: async (values: { name: string; file_id?: number }, ctx: any) => {
      try {
        await props.Storage.createDir(props.Crypto.keypair, values.name, props.Storage.dir?.id)
        ctx.resetForm()
        props.Storage.find(props.Crypto.keypair, props.Storage.dir?.id || undefined)
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
      title="Create a directory"
      button="info"
      buttonLabel="Create"
      has-cancel
      @cancel="$emit('cancel')"
      :form="form"
    >
      <p v-if="errorMessage" class="text-sm text-redish-900 dark:text-redish-200">
        {{ errorMessage }}
      </p>

      <AppField :form="form" label="Directory name" name="name" placeholder="Documents" autofocus />
    </CardBoxModal>
  </AppForm>
</template>
