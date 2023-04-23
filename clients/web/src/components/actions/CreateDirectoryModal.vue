<script setup lang="ts">
import { ref } from 'vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import { AppForm, AppField, AppButton } from '@/components/form'
import * as yup from 'yup'
import { store as storageStore } from '@/stores/storage'
import { store as cryptoStore } from '@/stores/crypto'
import type { ErrorResponse } from '@/stores/api'

const storage = storageStore()
const crypto = cryptoStore()

const props = defineProps<{
  modelValue: string | number | boolean
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
    onSubmit: async (values: { name: string; file_id?: number }) => {
      try {
        await storage.createDir(crypto.keypair, values.name, storage.dir?.id)
        storage.find(crypto.keypair, storage.dir?.id || null)
        emit('confirm')
        emit('update:modelValue', false)
        config.value.initialValues = { name: '' }
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
  <CardBoxModal
    :modelValue="props.modelValue"
    @update:modelValue="$emit('update:modelValue', $event)"
    title="Create a directory"
    button="info"
    buttonLabel="Create"
    has-cancel
    hide-submit
    @cancel="$emit('cancel')"
  >
    <AppForm v-if="config" :config="config" v-slot="{ form }">
      <p v-if="errorMessage" class="text-sm text-red-900 dark:text-red-200">
        {{ errorMessage }}
      </p>

      <AppField :form="form" label="Directory name" name="name" placeholder="Documents" autofocus />
      <AppButton :form="form" type="submit">Create</AppButton>
    </AppForm>
  </CardBoxModal>
</template>
