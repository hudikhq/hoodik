<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import FolderPicker from '@/components/ui/FolderPicker.vue'
import { AppForm, AppField } from '@/components/form'
import * as yup from 'yup'
import { createNote } from '!/storage/save'
import { emitFileTreeChange } from '!/storage/events'
import type { ErrorResponse } from '!/api'
import type { KeyPair } from 'types'

const LAST_FOLDER_KEY = 'hoodik:notes:lastFolder'

const props = defineProps<{
  modelValue?: boolean
  keypair: KeyPair
}>()

const emit = defineEmits(['update:modelValue', 'cancel', 'created'])

const router = useRouter()
const config = ref()
const errorMessage = ref()
const pickerRef = ref<InstanceType<typeof FolderPicker>>()

const startFolderId = ref<string | undefined>()
const startFolderName = ref<string | undefined>()
const folderId = ref<string | undefined>()
const folderName = ref('Root')

function restoreLastFolder() {
  try {
    const stored = localStorage.getItem(LAST_FOLDER_KEY)
    if (stored) {
      const { id, name } = JSON.parse(stored)
      startFolderId.value = id
      startFolderName.value = name || 'Root'
      folderId.value = id
      folderName.value = name || 'Root'
      return
    }
  } catch {
    // ignore corrupt storage
  }
  folderId.value = undefined
  folderName.value = 'Root'
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
      name: yup.string().required('File name is required')
    }),
    onSubmit: async (values: { name: string }, ctx: any) => {
      try {
        const file = await createNote(props.keypair, values.name, folderId.value)

        saveLastFolder()
        emitFileTreeChange({ type: 'created', folderId: folderId.value })
        ctx.resetForm()
        emit('created')
        emit('update:modelValue', false)
        router.push({ name: 'notes', params: { id: file.id } })
      } catch (err) {
        const error = err as ErrorResponse<unknown>
        config.value.initialErrors = error.validation || {}
        errorMessage.value = error.description || (err as Error).message
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
      title="New Note"
      button="info"
      buttonLabel="Create"
      has-cancel
      @cancel="$emit('cancel')"
      :form="form"
    >
      <p v-if="errorMessage" class="text-sm text-redish-900 dark:text-redish-200 mb-3">
        {{ errorMessage }}
      </p>

      <AppField :form="form" label="File name" name="name" placeholder="Untitled.md" autofocus />

      <div class="mt-4">
        <label class="block text-sm font-medium text-brownish-600 dark:text-brownish-300 mb-2">
          Folder
        </label>

        <FolderPicker
          ref="pickerRef"
          :keypair="keypair"
          :start-id="startFolderId"
          :start-name="startFolderName"
          @navigate="({ id, name }) => { folderId = id; folderName = name }"
        />

        <p class="mt-1 text-xs text-brownish-400">
          Note will be created in <strong>{{ folderName }}</strong>
        </p>
      </div>
    </CardBoxModal>
  </AppForm>
</template>
