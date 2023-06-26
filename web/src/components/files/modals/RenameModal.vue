<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import { AppForm, AppField } from '@/components/form'
import * as yup from 'yup'
import type { ErrorResponse } from '!/api'
import type { AppFile, CryptoStore, FilesStore } from 'types'

const props = defineProps<{
  modelValue?: AppFile | undefined
  Storage: FilesStore
  Crypto: CryptoStore
}>()

const emit = defineEmits(['update:modelValue', 'cancel', 'confirm'])

const file = computed({
  get() {
    return props.modelValue
  },
  set(value) {
    emit('update:modelValue', value)
  }
})

const config = ref()
const errorMessage = ref()

const init = () => {
  config.value = {
    initialValues: {
      name: file.value?.name
    },
    validationSchema: yup.object().shape({
      name: yup.string().required('New name is required')
    }),
    onSubmit: async (values: { name: string }, ctx: any) => {
      try {
        if (!file.value) throw new Error('File not found')

        await props.Storage.rename(props.Crypto.keypair, file.value, values.name)
        ctx.resetForm()
        emit('confirm')
        emit('update:modelValue', undefined)
      } catch (err) {
        const error = err as ErrorResponse<unknown>
        config.value.initialErrors = error.validation || {}
        errorMessage.value = error.description
      }
    }
  }
}

watch(() => props.modelValue, init, { immediate: true })
</script>

<template>
  <AppForm v-if="config" :config="config" v-slot="{ form }">
    <CardBoxModal
      :modelValue="!!file"
      @update:modelValue="$emit('update:modelValue', $event ? file : undefined)"
      :title="`Rename a ${file?.mime === 'dir' ? 'directory' : 'file'}`"
      button="info"
      buttonLabel="Rename"
      has-cancel
      @cancel="$emit('cancel')"
      :form="form"
    >
      <p v-if="errorMessage" class="text-sm text-redish-900 dark:text-redish-200">
        {{ errorMessage }}
      </p>

      <AppField :form="form" label="Name" name="name" placeholder="new name" autofocus />
    </CardBoxModal>
  </AppForm>
</template>
