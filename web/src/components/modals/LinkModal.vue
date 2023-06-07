<script setup lang="ts">
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import CardBoxComponentTitle from '@/components/ui/CardBoxComponentTitle.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import {
  mdiClose,
  mdiLink,
  mdiTrashCan,
  mdiOpenInNew,
  mdiDelete,
  mdiCheck,
  mdiPencil
} from '@mdi/js'
import { computed, ref, watch } from 'vue'
import type { FilesStore, KeyPair, LinksStore, AppLink, AppFile } from 'types'
import { formatPrettyDate, formatSize, localDateFromUtcString } from '!/index'
import PuppyLoader from '@/components/ui/PuppyLoader.vue'
import * as logger from '!/logger'
import { useRouter } from 'vue-router'
import { AppField, AppDateTime } from '@/components/form'
import * as cryptfns from '!/cryptfns'

const router = useRouter()

const props = defineProps<{
  modelValue: AppLink | AppFile | undefined
  Storage: FilesStore
  Links: LinksStore
  kp: KeyPair
}>()

const emits = defineEmits<{
  (event: 'update:modelValue', value: AppLink | AppFile | undefined): void
}>()

const loading = ref(false)
const loadingExpire = ref(false)
const loadedLink = ref()
const editExpire = ref(false)

const link = computed((): AppLink | undefined => {
  if (loadedLink.value) return loadedLink.value

  if (props.modelValue && (props.modelValue as AppLink)?.file_mime) {
    return props.modelValue as AppLink
  }

  return undefined
})

const file = computed((): AppFile | undefined => {
  if (props.modelValue && (props.modelValue as AppFile)?.mime) {
    return props.modelValue as AppFile
  }

  return undefined
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

const expiresAt = ref()
const linkExpiresAt = computed(() => {
  return link.value?.expires_at ? formatPrettyDate(link.value?.expires_at) : null
})

const isExpired = computed(() => {
  const now = new Date()

  return link.value?.expires_at && new Date(link.value?.expires_at) < now
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
  loadedLink.value = undefined
  editExpire.value = false
  emits('update:modelValue', undefined)
}

/**
 * On the load, try to get the link
 */
const getIfExists = async () => {
  if (!file.value) return
  if (link.value) return

  logger.debug('Getting link if it exists for file', file.value)
  loading.value = true

  if (file.value.link) {
    logger.debug('Getting link for file', file.value)
    const key = await cryptfns.rsa.decryptMessage(props.kp, file.value.link.encrypted_link_key)
    loadedLink.value = await props.Links.get(file.value.link.id, key)
  }

  loading.value = false
}

/**
 * Create a new link for the file if it doesn't exist
 */
const create = async () => {
  if (!file.value || link.value) return
  loading.value = true

  logger.debug('Creating link for file', file.value)
  loadedLink.value = await props.Links.create(file.value, props.kp)
  loading.value = false
}

/**
 * Delete the link
 */
const remove = async () => {
  if (!link.value) return
  loading.value = true

  if (!link.value) {
    logger.debug('No link to remove for link', link.value)
    return
  }

  logger.debug('Removing link for link', link.value)
  await props.Links.remove(link.value.id)
  props.Links.removeItem(link.value.id)
  loading.value = false
  cancel()
}

/**
 * Update the expiry date
 */
const updateExpiry = async (value: Date | undefined) => {
  if (!link.value) return
  loadingExpire.value = true

  logger.debug('Updating expiry for link', link.value)
  loadedLink.value = await props.Links.expire(link.value.id, value)
  expiresAt.value = value
  loadingExpire.value = false
  editExpire.value = false
}

watch(
  () => props.modelValue,
  () => getIfExists(),
  { immediate: true }
)

watch(
  () => link.value,
  (v) => {
    if (!v) return
    expiresAt.value = v.expires_at ? localDateFromUtcString(v.expires_at) : undefined
  }
)
</script>

<template>
  <CardBoxModal
    v-if="props.modelValue"
    :model-value="!!props.modelValue"
    :has-cancel="false"
    :hide-submit="true"
    @cancel="cancel"
  >
    <CardBoxComponentTitle :icon="mdiLink" title="Public link for a link">
      <BaseButton
        title="Close modal"
        :icon="mdiClose"
        color="dark"
        small
        rounded-full
        @click.prevent="cancel"
      />
    </CardBoxComponentTitle>

    <div class="ml-2">
      <BaseButton
        title="View link"
        :icon="mdiOpenInNew"
        color="dark"
        small
        rounded-full
        class="mr-2"
        :to="{ name: 'links-view', params: { link_id: link.id }, hash: `#${link.link_key_hex}` }"
        target="_blank"
        name="links-view"
        v-if="link"
      />
      <BaseButtonConfirm
        title="Delete public link"
        label="Delete public link"
        confirm-label="Confirm"
        cancel-label="Cancel"
        class="mr-1"
        :icon="mdiTrashCan"
        color="danger"
        xs
        rounded-full
        @confirm="remove"
        v-if="link"
      />
    </div>

    <div v-if="!loading">
      <div v-if="link">
        <div class="w-full p-2 border-b-[1px] border-brownish-700 text-redish-200" v-if="isExpired">
          <p>This link is expired, you can extend the expiry date below, or simply delete it.</p>
        </div>

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
          <div class="flex flex-col w-1/2">Downloads</div>
          <div class="flex flex-col w-1/2">{{ downloads }}</div>
        </div>
        <div class="flex flex-row p-2 border-b-[1px] border-brownish-700" v-if="!editExpire">
          <div class="flex flex-col w-6/12">
            <span class="mt-2"> Expires </span>
          </div>
          <div class="flex flex-col w-4/12">
            <span class="mt-2">
              {{ linkExpiresAt || 'n/a' }}
            </span>
          </div>
          <div class="flex flex-col w-2/12">
            <div class="flex justify-end mt-0.5 pl-2">
              <BaseButton
                title="Remove expiry"
                :icon="mdiDelete"
                color="dark"
                :xs="true"
                rounded-full
                class="ml-2 mt-0.5"
                :disabled="!expiresAt || loadingExpire"
                @click.prevent="updateExpiry(undefined)"
              />
              <BaseButton
                title="Close modal"
                :icon="mdiPencil"
                color="dark"
                :xs="true"
                rounded-full
                class="ml-2 mt-0.5"
                @click.prevent="editExpire = true"
                :disabled="loadingExpire"
              />
            </div>
          </div>
        </div>
        <div class="flex flex-row p-2 border-b-[1px] border-brownish-700" v-else>
          <div class="flex flex-col w-10/12">
            <AppDateTime
              v-model="expiresAt"
              name="expires-at"
              :disabled="loadingExpire"
              :min="new Date()"
            />
          </div>
          <div class="flex flex-col w-2/12 mt-1">
            <div class="flex justify-end mt-0.5 pl-2">
              <BaseButton
                title="Save changes"
                :icon="mdiCheck"
                color="dark"
                :xs="true"
                rounded-full
                :disabled="!expiresAt || loadingExpire"
                @click.prevent="updateExpiry(expiresAt)"
              />
              <BaseButton
                title="Remove expiry"
                :icon="mdiDelete"
                color="dark"
                :xs="true"
                rounded-full
                class="ml-2"
                :disabled="!expiresAt || loadingExpire"
                @click.prevent="updateExpiry(undefined)"
              />
              <BaseButton
                title="Cancel editing"
                :icon="mdiClose"
                color="dark"
                :xs="true"
                rounded-full
                class="ml-2"
                :disabled="loadingExpire"
                @click.prevent="editExpire = false"
              />
            </div>
          </div>
        </div>
        <div class="flex flex-row p-2 border-b-[1px] border-brownish-700">
          <div class="flex flex-col w-full">
            <AppField
              v-if="linkUrl"
              name="link"
              label="Link"
              v-model="linkUrl"
              :allow-copy="true"
            />
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
<style scoped></style>
