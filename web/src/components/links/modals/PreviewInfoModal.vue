<script lang="ts" setup>
import CardBoxComponentTitle from '@/components/ui/CardBoxComponentTitle.vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import SpinnerIcon from '@/components/ui/SpinnerIcon.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { AppField } from '@/components/form'
import { mdiLink, mdiClose } from '@mdi/js'
import { computed, ref, watch } from 'vue'
import * as cryptfns from '!/cryptfns'
import type { AppLink } from 'types'

const props = defineProps<{
  modelValue: AppLink | undefined
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: AppLink | undefined): void
}>()

const link = computed({
  get: () => props.modelValue,
  set: (value: AppLink | undefined) => emits('update:modelValue', value)
})

const signatureValid = ref(false)
const loading = ref(false)

const cancel = () => {
  link.value = undefined
}

watch(
  link,
  async () => {
    if (!link.value) return

    loading.value = true

    signatureValid.value = await cryptfns.rsa.verify(
      link.value.signature,
      link.value.file_id,
      link.value.owner_pubkey
    )

    setTimeout(() => {
      loading.value = false
    }, 1)
  },
  { immediate: true }
)
</script>
<template>
  <CardBoxModal
    v-if="link"
    :model-value="!!link"
    :has-cancel="false"
    :hide-submit="true"
    @cancel="cancel"
  >
    <CardBoxComponentTitle :icon="mdiLink" title="Link details">
      <BaseButton
        title="Close modal"
        :icon="mdiClose"
        color="dark"
        small
        rounded-full
        @click.prevent="cancel"
      />
    </CardBoxComponentTitle>

    <div v-if="link">
      <div class="flex flex-row p-2 border-b-[1px] border-brownish-700" v-if="loading">
        <div class="flex w-full">
          <SpinnerIcon class="w-6 h-6 mr-2" />
          <span>Verifying signature...</span>
        </div>
      </div>
      <div class="flex flex-row p-2 border-b-[1px] border-brownish-700" v-else>
        <div class="flex flex-col w-full text-greeny-400" v-if="signatureValid">
          This link has been signed by the owner, {{ link.owner_email }}.
        </div>
        <div class="flex flex-col w-full text-redish-400" v-else>
          The signature of this link could not be verified and may be invalid, please proceed
          cautiously.
        </div>
      </div>

      <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
        <div class="w-full">
          <AppField
            name="owner_pubkey"
            type="text"
            v-model="link.file_id"
            label="File ID"
            :allow-copy="true"
            :disabled="true"
          />
        </div>
      </div>
      <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
        <div class="w-full">
          <AppField
            name="owner_pubkey"
            :textarea="true"
            v-model="link.owner_pubkey"
            label="Owner Public Key"
            :allow-copy="true"
            :disabled="true"
          />
        </div>
      </div>
      <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
        <div class="w-full">
          <AppField
            name="signature"
            :textarea="true"
            v-model="link.signature"
            label="Owner signature"
            :allow-copy="true"
            :disabled="true"
          />
        </div>
      </div>
    </div>
  </CardBoxModal>
</template>
