<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import FormError from '@/components/ui/FormError.vue'
import FolderPicker from '@/components/ui/FolderPicker.vue'
import { AppForm, AppField } from '@/components/form'
import * as yup from 'yup'
import { createNote } from '!/storage/save'
import { emitFileTreeChange } from '!/storage/events'
import type { ErrorResponse } from '!/api'
import type { KeyPair } from 'types'
import { humanizeError } from '!/index'

const LAST_FOLDER_KEY = 'hoodik:notes:lastFolder'

const props = defineProps<{
  modelValue?: boolean
  keypair: KeyPair
  authenticatedUserId?: string
}>()

const emit = defineEmits(['update:modelValue', 'cancel', 'created'])

const router = useRouter()
const { t } = useI18n()
const config = ref()
const errorMessage = ref()
const pickerRef = ref<InstanceType<typeof FolderPicker>>()

const startFolderId = ref<string | undefined>()
const startFolderName = ref<string | undefined>()
const folderId = ref<string | undefined>()
const folderName = ref(t('notes.create.rootFolder'))

function restoreLastFolder() {
  try {
    const stored = localStorage.getItem(LAST_FOLDER_KEY)
    if (stored) {
      const { id, name } = JSON.parse(stored)
      startFolderId.value = id
      startFolderName.value = name || t('notes.create.rootFolder')
      folderId.value = id
      folderName.value = name || t('notes.create.rootFolder')
      return
    }
  } catch {
    // ignore corrupt storage
  }
  folderId.value = undefined
  folderName.value = t('notes.create.rootFolder')
}

function saveLastFolder() {
  localStorage.setItem(
    LAST_FOLDER_KEY,
    JSON.stringify({ id: folderId.value, name: folderName.value })
  )
}

const init = () => {
  errorMessage.value = undefined
  restoreLastFolder()

  config.value = {
    initialValues: {
      name: 'Untitled.md'
    },
    validationSchema: yup.object().shape({
      name: yup.string().required(t('notes.create.nameRequired'))
    }),
    onSubmit: async (values: { name: string }, ctx: any) => {
      try {
        const file = await createNote(
          props.keypair,
          values.name,
          folderId.value,
          props.authenticatedUserId
        )

        saveLastFolder()
        emitFileTreeChange({ type: 'created', folderId: folderId.value })
        ctx.resetForm()
        emit('created')
        emit('update:modelValue', false)
        router.push({ name: 'notes', params: { id: file.id } })
      } catch (err) {
        const error = err as ErrorResponse<unknown>
        config.value.initialErrors = error.validation || {}
        errorMessage.value = humanizeError(err)
      }
    }
  }
}

onMounted(() => init())
</script>

<template>
  <AppForm v-if="config" :config="config" v-slot="{ form }">
    <CardBoxModal
      :modelValue="props.modelValue"
      @update:modelValue="(v) => { $emit('update:modelValue', v); if (v) init(); }"
      :title="$t('notes.create.title')"
      button="info"
      :buttonLabel="$t('common.create')"
      has-cancel
      @cancel="$emit('cancel')"
      :form="form"
    >
      <FormError v-if="errorMessage">{{ errorMessage }}</FormError>

      <AppField :form="form" :label="$t('notes.create.nameLabel')" name="name" placeholder="Untitled.md" autofocus />

      <div class="mt-4">
        <label class="block text-sm font-medium text-brownish-600 dark:text-brownish-50 mb-2">
          {{ $t('notes.create.folderLabel') }}
        </label>

        <FolderPicker
          ref="pickerRef"
          :keypair="keypair"
          :start-id="startFolderId"
          :start-name="startFolderName"
          @navigate="({ id, name }) => { folderId = id; folderName = name }"
        />

        <i18n-t keypath="notes.create.locationHint" tag="p" class="mt-1 text-xs text-brownish-400 dark:text-brownish-50">
          <template #folder>
            <strong>{{ folderName }}</strong>
          </template>
        </i18n-t>
      </div>
    </CardBoxModal>
  </AppForm>
</template>
