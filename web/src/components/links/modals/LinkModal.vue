<script setup lang="ts">
import { computed, ref } from 'vue'
import { mdiClose, mdiLink, mdiAccountPlus } from '@mdi/js'

import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import SharingModal from '@/components/shares/SharingModal.vue'
import SharingLinkPanel from '@/components/links/SharingLinkPanel.vue'

import { useCapability } from '@/composables/useCapability'
import { store as loginStore } from '!/auth/login'

import type { FilesStore, KeyPair, LinksStore, AppLink, AppFile } from 'types'

const props = defineProps<{
  modelValue: AppLink | AppFile | undefined
  Storage: FilesStore
  Links: LinksStore
  kp: KeyPair
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: AppLink | AppFile | undefined): void
}>()

const { sharingEnabled } = useCapability()
const login = loginStore()
const sharingModalFor = ref<AppFile | null>(null)

const link = computed((): AppLink | undefined => {
  if (props.modelValue && (props.modelValue as AppLink)?.file_mime) {
    return props.modelValue as AppLink
  }
  return undefined
})

function openShareWithAccount(): void {
  const referenceFile = (props.modelValue && (props.modelValue as AppFile).mime)
    ? (props.modelValue as AppFile)
    : props.Storage.items.find((f) => f.id === link.value?.file_id) ?? null
  if (!referenceFile) return
  sharingModalFor.value = referenceFile
}

const cancel = () => {
  emits('update:modelValue', undefined)
}
</script>

<template>
  <CardBoxModal
    v-if="props.modelValue"
    :model-value="!!props.modelValue"
    :has-cancel="false"
    :hide-submit="true"
    @cancel="cancel"
  >
    <div class="flex items-center justify-between gap-3 mb-4">
      <div class="flex items-center gap-2 min-w-0">
        <BaseIcon
          :path="mdiLink"
          :size="24"
          class="shrink-0 text-brownish-300 dark:text-brownish-200"
        />
        <h2 class="text-lg sm:text-xl font-semibold truncate">Public link</h2>
      </div>
      <button
        type="button"
        class="shrink-0 w-11 h-11 inline-flex items-center justify-center rounded-full text-brownish-400 hover:text-brownish-100 hover:bg-brownish-100 dark:hover:bg-brownish-800 transition-colors"
        title="Close"
        @click.prevent="cancel"
      >
        <BaseIcon :path="mdiClose" :size="20" />
      </button>
    </div>

    <div v-if="link && sharingEnabled" class="mb-3">
      <BaseButton
        title="Share with a Hoodik account instead"
        label="Share with a Hoodik account"
        :icon="mdiAccountPlus"
        color="dark"
        small
        data-testid="link-modal-share-account"
        @click.prevent="openShareWithAccount"
      />
    </div>

    <SharingLinkPanel
      :source="props.modelValue"
      :storage="props.Storage"
      :links="props.Links"
      :kp="props.kp"
    />
  </CardBoxModal>
  <SharingModal
    v-if="sharingModalFor && login.authenticated"
    :file="sharingModalFor"
    :authenticated-user-id="login.authenticated.user.id"
    :keypair="kp"
    :storage="props.Storage"
    :links="props.Links"
    @close="() => { sharingModalFor = null }"
  />
</template>
<style scoped></style>
