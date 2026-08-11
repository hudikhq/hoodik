<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import FormError from '@/components/ui/FormError.vue'
import { AppForm, AppField } from '@/components/form'
import * as yup from 'yup'
import type { ErrorResponse } from '!/api'
import type { AppFile, CryptoStore, FilesStore } from 'types'
import { humanizeError } from '!/index'

const props = defineProps<{
  modelValue?: AppFile | undefined
  Storage: FilesStore
  Crypto: CryptoStore
}>()

const emit = defineEmits(['update:modelValue', 'cancel', 'confirm'])

const { t } = useI18n()

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
      name: yup.string().required(t('files.rename.nameRequired'))
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
        errorMessage.value = humanizeError(err)
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
      :title="
        file?.mime === 'dir' ? $t('files.rename.directoryTitle') : $t('files.rename.fileTitle')
      "
      button="info"
      :buttonLabel="$t('common.rename')"
      has-cancel
      @cancel="$emit('cancel')"
      :form="form"
    >
      <FormError v-if="errorMessage">{{ errorMessage }}</FormError>

      <AppField
        :form="form"
        :label="$t('common.name')"
        name="name"
        :placeholder="$t('files.rename.namePlaceholder')"
        autofocus
      />
    </CardBoxModal>
  </AppForm>
</template>
