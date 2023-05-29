<script setup lang="ts">
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import CardBoxComponentTitle from '@/components/ui/CardBoxComponentTitle.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import { mdiClose, mdiLink, mdiTrashCan, mdiOpenInNew } from '@mdi/js'
import { computed, ref } from 'vue'
import type { FilesStore, KeyPair, LinksStore, AppLink } from 'types'
import { formatPrettyDate, formatSize } from '!/index'
import PuppyLoader from '@/components/ui/PuppyLoader.vue'
import * as logger from '!/logger'
import { useRouter } from 'vue-router'
import { AppField } from '@/components/form'

const router = useRouter()

const props = defineProps<{
  modelValue: AppLink | undefined
  Storage: FilesStore
  Links: LinksStore
  kp: KeyPair
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: AppLink | undefined): void
}>()

const loading = ref(false)

const link = computed({
  get: () => props.modelValue,
  set: (value: AppLink | undefined) => emits('update:modelValue', value)
})

const size = computed(() => {
  if (!link.value) return '-'
  return formatSize(link.value.file_size)
})

const created = computed(() => {
  if (!link.value) return ''
  return link.value?.created_at ? formatPrettyDate(link.value?.created_at) : ''
})

const downloads = computed(() => {
  if (!link.value) return null
  return link.value.downloads || 0
})

const fileCreatedAt = computed(() => {
  if (!link.value) return null
  return link.value?.file_created_at ? formatPrettyDate(link.value?.file_created_at) : ''
})

const linkExpiresAt = computed(() => {
  if (!link.value) return null
  return link.value?.expires_at ? formatPrettyDate(link.value?.expires_at) : null
})

const linkUrl = computed(() => {
  if (!link.value) return null

  const linkKeyHex = link.value.link_key_hex || ''
  const route = router.resolve({
    name: 'links-view',
    params: {
      link_id: link.value.id
    },
    hash: `#${linkKeyHex}`
  })

  const url = new URL(route.href, window.location.origin).href

  return `${url}`
})

const cancel = () => {
  emits('update:modelValue', undefined)
}

const remove = async () => {
  if (!link.value) return
  loading.value = true

  if (!link.value) {
    logger.debug('No link to remove for link', link.value)
    return
  }

  logger.debug('Removing link for link', link.value)
  await props.Links.del(link.value.id)
  props.Links.removeItem(link.value.id)
  link.value = undefined
  loading.value = false
}
</script>

<template>
  <CardBoxModal
    v-if="link"
    :model-value="!!link"
    :has-cancel="false"
    :hide-submit="true"
    @cancel="cancel"
  >
    <CardBoxComponentTitle :icon="mdiLink" title="Public link for a link">
      <div>
        <BaseButtonConfirm
          title="Delete public link"
          label="Delete public link"
          confirm-label="Confirm"
          cancel-label="Cancel"
          class="mr-2"
          :icon="mdiTrashCan"
          color="danger"
          xs
          rounded-full
          @confirm="remove"
        />
        <BaseButton
          title="View link"
          :icon="mdiOpenInNew"
          color="dark"
          small
          rounded-full
          class="mr-2"
          :to="{ name: 'links-view', params: { link_id: link.id }, hash: `#${link.link_key_hex}` }"
          target="_blank"
        />
        <BaseButton
          title="Close modal"
          :icon="mdiClose"
          color="dark"
          small
          rounded-full
          @click.prevent="cancel"
        />
      </div>
    </CardBoxComponentTitle>

    <div v-if="!loading">
      <div>
        <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
          <div class="flex flex-col w-1/2">Name</div>
          <div class="flex flex-col w-1/2">{{ link.name }}</div>
        </div>
        <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
          <div class="flex flex-col w-1/2">Type</div>
          <div class="flex flex-col w-1/2">{{ link.file_mime }}</div>
        </div>
        <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
          <div class="flex flex-col w-1/2">Size</div>
          <div class="flex flex-col w-1/2">{{ size }}</div>
        </div>
        <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
          <div class="flex flex-col w-1/2">File Created</div>
          <div class="flex flex-col w-1/2">{{ created }}</div>
        </div>
        <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
          <div class="flex flex-col w-1/2">Link Created</div>
          <div class="flex flex-col w-1/2">{{ fileCreatedAt }}</div>
        </div>
        <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
          <div class="flex flex-col w-1/2">Downloads through link</div>
          <div class="flex flex-col w-1/2">{{ downloads }}</div>
        </div>
        <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
          <div class="flex flex-col w-1/2">Link expires</div>
          <div class="flex flex-col w-1/2">{{ linkExpiresAt || 'n/a' }}</div>
        </div>
        <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
          <div class="flex flex-col w-full">
            <AppField name="link" label="Link" v-model="linkUrl" :allow-copy="true" />
          </div>
        </div>
      </div>
    </div>
    <PuppyLoader v-else />
  </CardBoxModal>
</template>
