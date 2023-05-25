<script setup lang="ts">
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import CardBoxComponentTitle from '@/components/ui/CardBoxComponentTitle.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import { mdiClose, mdiLink, mdiTrashCan } from '@mdi/js'
import { computed, ref, watch } from 'vue'
import type { FilesStore, KeyPair, LinksStore, ListAppFile } from 'types'
import { formatPrettyDate, formatSize } from '!/index'
import * as cryptfns from '!/cryptfns'
import PuppyLoader from '@/components/ui/PuppyLoader.vue'
import * as logger from '!/logger'
import { useRouter } from 'vue-router'
import { AppField } from '@/components/form'

const router = useRouter()

const props = defineProps<{
  modelValue: ListAppFile | undefined
  Storage: FilesStore
  Links: LinksStore
  kp: KeyPair
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: ListAppFile | undefined): void
}>()

const loading = ref(false)

const file = computed({
  get: () => props.modelValue,
  set: (value: ListAppFile | undefined) => emits('update:modelValue', value)
})

const size = computed(() => {
  if (!file.value) return '-'
  if (file.value.mime === 'dir') return '-'
  return formatSize(file.value.size)
})

const created = computed(() => {
  if (!file.value) return ''
  return file.value?.file_created_at ? formatPrettyDate(file.value?.file_created_at) : ''
})

const downloads = computed(() => {
  if (!file.value || !file.value.link) return null
  return file.value.link?.downloads || 0
})

const linkCreatedAt = computed(() => {
  if (!file.value || !file.value.link) return null
  return file.value?.link?.created_at ? formatPrettyDate(file.value?.link?.created_at) : ''
})

const linkExpiresAt = computed(() => {
  if (!file.value || !file.value.link) return null
  return file.value?.link?.expires_at ? formatPrettyDate(file.value?.link?.expires_at) : null
})

const linkUrl = computed(() => {
  if (!file.value || !file.value.link) return null

  const linkKeyHex = file.value.link?.link_key_hex || ''
  const route = router.resolve({
    name: 'links-view',
    params: {
      link_id: file.value.link.id
    },
    hash: `#${linkKeyHex}`
  })

  const url = new URL(route.href, window.location.origin).href

  return `${url}`
})

const cancel = () => {
  emits('update:modelValue', undefined)
}

/**
 * On the load, try to get the link
 */
const getIfExists = async () => {
  if (!file.value) return
  loading.value = true

  if (file.value.link) {
    logger.debug('Getting link for file', file.value)
    const key = await cryptfns.rsa.decryptMessage(props.kp, file.value.link.encrypted_link_key)
    const link = await props.Links.get(file.value.link.id, key)
    file.value.link = { ...link, user_id: link.owner_id }
    props.Storage.upsertItem(file.value)
  }

  loading.value = false
}

/**
 * Create a new link for the file if it doesn't exist
 */
const create = async () => {
  if (!file.value || file.value?.link) return
  loading.value = true

  logger.debug('Creating link for file', file.value)
  const link = await props.Links.create(file.value, props.kp)
  file.value.link = { ...link, user_id: link.owner_id }
  props.Storage.upsertItem(file.value)
  loading.value = false
}

const remove = async () => {
  if (!file.value) return
  loading.value = true

  if (!file.value.link) {
    logger.debug('No link to remove for file', file.value)
    return
  }

  logger.debug('Removing link for file', file.value)
  await props.Links.del(file.value.link.id)
  file.value.link = undefined
  props.Storage.upsertItem(file.value)
  loading.value = false
  file.value = undefined
}

watch(
  () => props.modelValue,
  () => getIfExists(),
  { immediate: true }
)
</script>

<template>
  <CardBoxModal
    v-if="file"
    :model-value="!!file"
    :has-cancel="false"
    :hide-submit="true"
    @cancel="cancel"
  >
    <CardBoxComponentTitle :icon="mdiLink" title="Public link for a file">
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
          v-if="file.link"
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
      <div v-if="file.link">
        <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
          <div class="flex flex-col w-1/2">Name</div>
          <div class="flex flex-col w-1/2">{{ file.metadata?.name }}</div>
        </div>
        <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
          <div class="flex flex-col w-1/2">Type</div>
          <div class="flex flex-col w-1/2">{{ file.mime }}</div>
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
          <div class="flex flex-col w-1/2">{{ linkCreatedAt }}</div>
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
      <div v-else>
        <p>It seems like this file doesn't have a public link, would you like to create one?</p>

        <div class="flex flex-col items-start mt-6">
          <BaseButton
            title="Create link"
            label="Create link"
            color="dark"
            small
            rounded-full
            @click.prevent="create"
          />
        </div>
      </div>
    </div>
    <PuppyLoader v-else />
  </CardBoxModal>
</template>
