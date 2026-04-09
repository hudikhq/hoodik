<script setup lang="ts">
import { ref, nextTick } from 'vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import { AppForm, AppField } from '@/components/form'
import * as yup from 'yup'
import { createNote } from '!/storage/save'
import { emitFileTreeChange } from '!/storage/events'
import { useRouter } from 'vue-router'
import type { ErrorResponse } from '!/api'
import type { CryptoStore, FilesStore } from 'types'

const props = defineProps<{
  modelValue?: boolean | undefined
  Storage: FilesStore
  Crypto: CryptoStore
}>()

const emit = defineEmits(['update:modelValue', 'cancel', 'confirm'])

const router = useRouter()
const config = ref()
const errorMessage = ref()

const init = () => {
  config.value = {
    initialValues: {
      name: 'Untitled.md'
    },
    validationSchema: yup.object().shape({
      name: yup.string().required('File name is required')
    }),
    onSubmit: async (values: { name: string }, ctx: any) => {
      try {
        const folderId = props.Storage.dir?.id
        const file = await createNote(props.Crypto.keypair, values.name, folderId)

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
        errorMessage.value = error.description || (err as Error).message
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
      title="Create a new file"
      button="info"
      buttonLabel="Create"
      has-cancel
      @cancel="$emit('cancel')"
      :form="form"
    >
      <p v-if="errorMessage" class="text-sm text-redish-900 dark:text-redish-200">
        {{ errorMessage }}
      </p>

      <AppField :form="form" label="File name" name="name" placeholder="Untitled.md" autofocus />
    </CardBoxModal>
  </AppForm>
</template>
