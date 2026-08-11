<script lang="ts" setup>
import CardBoxComponentTitle from '@/components/ui/CardBoxComponentTitle.vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import SpinnerIcon from '@/components/ui/SpinnerIcon.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import { AppField } from '@/components/form'
import { mdiLink, mdiClose } from '@mdi/js'
import { computed, ref, watch } from 'vue'
import { verifyOwnerSignature } from '!/links/crypto'
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

    signatureValid.value = await verifyOwnerSignature(link.value)

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
    <CardBoxComponentTitle :icon="mdiLink" :title="$t('links.details.title')">
      <BaseButton
        :title="$t('links.details.closeModal')"
        :icon="mdiClose"
        color="dark"
        small
        rounded-full
        @click.prevent="cancel"
      />
    </CardBoxComponentTitle>

    <div v-if="link">
      <div class="flex flex-row p-2 border-b-[1px] border-paper-200 dark:border-brownish-700" v-if="loading">
        <div class="flex w-full">
          <SpinnerIcon class="w-6 h-6 mr-2" />
          <span>{{ $t('links.details.verifyingSignature') }}</span>
        </div>
      </div>
      <div class="flex flex-row p-2 border-b-[1px] border-paper-200 dark:border-brownish-700" v-else>
        <div class="flex flex-col w-full text-greeny-500 dark:text-greeny-300" v-if="signatureValid">
          {{ $t('links.details.signatureValid', { email: link.owner_email }) }}
        </div>
        <div class="flex flex-col w-full text-redish-400 dark:text-redish-100" v-else>
          {{ $t('links.details.signatureInvalid') }}
        </div>
      </div>

      <div class="flex flex-row p-2 border-b-[1px] border-paper-200 dark:border-brownish-700">
        <div class="w-full">
          <AppField
            name="owner_pubkey"
            type="text"
            v-model="link.file_id"
            :label="$t('links.details.fileId')"
            :allow-copy="true"
            :disabled="true"
          />
        </div>
      </div>
      <div class="flex flex-row p-2 border-b-[1px] border-paper-200 dark:border-brownish-700">
        <div class="w-full">
          <AppField
            name="owner_pubkey"
            :textarea="true"
            v-model="link.owner_pubkey"
            :label="$t('links.details.ownerPublicKey')"
            :allow-copy="true"
            :disabled="true"
          />
        </div>
      </div>
      <div class="flex flex-row p-2 border-b-[1px] border-paper-200 dark:border-brownish-700">
        <div class="w-full">
          <AppField
            name="signature"
            :textarea="true"
            v-model="link.signature"
            :label="$t('links.details.ownerSignature')"
            :allow-copy="true"
            :disabled="true"
          />
        </div>
      </div>
    </div>
  </CardBoxModal>
</template>
