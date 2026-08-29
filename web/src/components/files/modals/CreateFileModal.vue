<script setup lang="ts">
import { ref, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import FormError from '@/components/ui/FormError.vue'
import { AppForm, AppField } from '@/components/form'
import * as yup from 'yup'
import { createNote } from '!/storage/save'
import { emitFileTreeChange } from '!/storage/events'
import { useRouter } from 'vue-router'
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

const router = useRouter()
const config = ref()
const errorMessage = ref()

const init = () => {
  config.value = {
    initialValues: {
      name: 'Untitled.md'
    },
    validationSchema: yup.object().shape({
      name: yup.string().required(t('files.createFile.nameRequired'))
    }),
    onSubmit: async (values: { name: string }, ctx: any) => {
      try {
        const parent = props.Storage.dir ?? null
        const folderId = parent?.id
        const file = await createNote(
          props.Crypto.keypair,
          values.name,
          parent,
          props.authenticatedUserId,
          parent ? await props.Storage.writeRosterId(parent.id) : undefined
        )

        emitFileTreeChange({ type: 'created', folderId })
        props.Storage.find(props.Crypto.keypair, folderId || undefined)

        ctx.resetForm()
        emit('confirm')
        emit('update:modelValue', false)

        await nextTick()
        router.push({ name: 'notes', params: { id: file.id } })
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
      :title="$t('files.createFile.title')"
      button="info"
      :buttonLabel="$t('common.create')"
      has-cancel
      @cancel="$emit('cancel')"
      :form="form"
    >
      <FormError v-if="errorMessage">{{ errorMessage }}</FormError>

      <AppField
        :form="form"
        :label="$t('files.createFile.nameLabel')"
        name="name"
        placeholder="Untitled.md"
        autofocus
      />
    </CardBoxModal>
  </AppForm>
</template>
