<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import {
  mdiTrashCan,
  mdiOpenInNew,
  mdiDelete,
  mdiCheck,
  mdiPencil,
  mdiClose
} from '@mdi/js'

import BaseButton from '@/components/ui/BaseButton.vue'
import BaseButtonConfirm from '@/components/ui/BaseButtonConfirm.vue'
import PuppyLoader from '@/components/ui/PuppyLoader.vue'
import { AppField, AppDateTime } from '@/components/form'

import { formatPrettyDate, formatSize, localDateFromUtcString } from '!/index'
import * as logger from '!/logger'
import { unwrapOwnLinkKey } from '!/links/crypto'
import { useRouter } from 'vue-router'

import type {
  AppFile,
  AppLink,
  FilesStore,
  KeyPair,
  LinksStore
} from 'types'

const router = useRouter()

const props = defineProps<{
  /**
   * Source the panel reads from. Either a file (no link yet — create CTA),
   * or an existing AppLink (manage view), or undefined (empty placeholder
   * for an unloaded file).
   */
  source: AppFile | AppLink | undefined
  storage: FilesStore
  links: LinksStore
  kp: KeyPair
  /** When true, the panel renders the existing link / create CTA but
   *  every action is disabled with no inline explanation — the parent
   *  surface owns the rationale copy. */
  readOnly?: boolean
}>()

const emit = defineEmits<{
  (e: 'created', link: AppLink): void
  (e: 'removed', linkId: string): void
}>()

const loading = ref(false)
const loadingExpire = ref(false)
const loadedLink = ref<AppLink | undefined>()
const editExpire = ref(false)
const expiresAt = ref<Date | undefined>()

const file = computed((): AppFile | undefined => {
  if (props.source && (props.source as AppFile)?.mime) {
    return props.source as AppFile
  }
  return undefined
})

const link = computed((): AppLink | undefined => {
  if (loadedLink.value) return loadedLink.value
  if (props.source && (props.source as AppLink)?.file_mime) {
    return props.source as AppLink
  }
  return undefined
})

const size = computed(() => (link.value ? formatSize(link.value.file_size) : '-'))
const created = computed(() =>
  link.value?.created_at ? formatPrettyDate(link.value.created_at) : ''
)
const downloads = computed(() => link.value?.downloads ?? 0)
const fileModifiedAt = computed(() =>
  link.value?.file_modified_at ? formatPrettyDate(link.value.file_modified_at) : ''
)
const linkExpiresAt = computed(() =>
  link.value?.expires_at ? formatPrettyDate(link.value.expires_at) : null
)
const isExpired = computed(() => {
  const now = new Date().valueOf() / 1000
  return link.value?.expires_at != null && link.value.expires_at < now
})

const linkUrl = computed(() => {
  if (!link.value) return null
  const linkKeyHex = link.value.link_key_hex
  if (!linkKeyHex) return null
  // Defensive against test mounts without a router plugin — the
  // production app always mounts under one, but vitest harnesses
  // sometimes don't and `useRouter()` returns undefined there.
  if (!router) return null
  const route = router.resolve({
    name: 'links-view',
    params: { link_id: link.value.id },
    hash: `#${linkKeyHex}`
  })
  return new URL(route.href, window.location.origin).href
})

/**
 * On the load, try to get the link
 */
const getIfExists = async () => {
  if (!file.value) return
  if (link.value) return

  logger.debug('Getting link if it exists for file', file.value)
  loading.value = true
  try {
    if (file.value.link) {
      const key = await unwrapOwnLinkKey(file.value.link.encrypted_link_key, props.kp)
      loadedLink.value = await props.links.get(file.value.link.id, key)
    }
  } finally {
    loading.value = false
  }
}

const create = async () => {
  if (!file.value || link.value || props.readOnly) return
  loading.value = true
  try {
    const fresh = await props.links.create(file.value, props.kp)
    // `links.create` returns the decrypted link but doesn't fan out into
    // the Pinia store — surfaces that read from `Links.items` (the
    // grants store, the public-links view) stay stale until we upsert
    // here.
    props.links.upsertItem(fresh)
    loadedLink.value = fresh
    await props.storage.find(props.kp, file.value.file_id ?? undefined)
    emit('created', fresh)
  } finally {
    loading.value = false
  }
}

const remove = async () => {
  if (!link.value || props.readOnly) return
  loading.value = true
  try {
    const removedId = link.value.id
    await props.links.remove(removedId)
    props.links.removeItem(removedId)
    loadedLink.value = undefined
    emit('removed', removedId)
  } finally {
    loading.value = false
  }
}

const updateExpiry = async (value: Date | undefined) => {
  if (!link.value || props.readOnly) return
  loadingExpire.value = true
  try {
    loadedLink.value = await props.links.expire(link.value.id, value)
    expiresAt.value = value
    editExpire.value = false
  } finally {
    loadingExpire.value = false
  }
}

watch(() => props.source, () => { void getIfExists() }, { immediate: true })

watch(
  () => link.value,
  (v) => {
    if (!v) return
    expiresAt.value = v.expires_at ? localDateFromUtcString(v.expires_at) : undefined
  }
)
</script>

<template>
  <div v-if="!loading">
    <div v-if="link" class="space-y-3">
      <div
        v-if="isExpired"
        class="px-3 py-2 rounded-lg bg-redish-100 dark:bg-redish-900/40 text-redish-700 dark:text-redish-200 text-sm"
      >
        This link has expired. Extend its expiry below, or delete it.
      </div>

      <div v-if="linkUrl" class="space-y-1.5" data-testid="sharing-link-url">
        <AppField
          name="link"
          label="Public URL"
          v-model="linkUrl"
          :allow-copy="true"
        />
        <div class="flex flex-wrap items-center gap-2">
          <BaseButton
            title="Open the link in a new tab"
            label="Open"
            :icon="mdiOpenInNew"
            color="dark"
            small
            :to="{ name: 'links-view', params: { link_id: link.id }, hash: `#${link.link_key_hex}` }"
            target="_blank"
            name="links-view"
          />
          <BaseButtonConfirm
            title="Delete public link"
            label="Delete link"
            confirm-label="Confirm"
            cancel-label="Cancel"
            :icon="mdiTrashCan"
            color="danger"
            small
            :disabled="readOnly"
            data-testid="sharing-link-remove"
            @confirm="remove"
          />
        </div>
      </div>

      <dl
        class="grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-2 px-3 py-3 rounded-lg bg-brownish-50 dark:bg-brownish-900/60 border border-brownish-200 dark:border-brownish-700 text-sm"
      >
        <div class="flex justify-between sm:flex-col gap-1">
          <dt class="text-xs uppercase tracking-wider text-brownish-300">Name</dt>
          <dd class="truncate text-right sm:text-left" :title="link.name">{{ link.name }}</dd>
        </div>
        <div class="flex justify-between sm:flex-col gap-1">
          <dt class="text-xs uppercase tracking-wider text-brownish-300">Type</dt>
          <dd class="truncate text-right sm:text-left">{{ link.file_mime }}</dd>
        </div>
        <div class="flex justify-between sm:flex-col gap-1">
          <dt class="text-xs uppercase tracking-wider text-brownish-300">Size</dt>
          <dd class="text-right sm:text-left">{{ size }}</dd>
        </div>
        <div class="flex justify-between sm:flex-col gap-1">
          <dt class="text-xs uppercase tracking-wider text-brownish-300">Downloads</dt>
          <dd class="text-right sm:text-left">{{ downloads }}</dd>
        </div>
        <div class="flex justify-between sm:flex-col gap-1">
          <dt class="text-xs uppercase tracking-wider text-brownish-300">File created</dt>
          <dd class="text-right sm:text-left">{{ created }}</dd>
        </div>
        <div class="flex justify-between sm:flex-col gap-1">
          <dt class="text-xs uppercase tracking-wider text-brownish-300">Link created</dt>
          <dd class="text-right sm:text-left">{{ fileModifiedAt }}</dd>
        </div>
      </dl>

      <div
        class="flex items-center justify-between gap-2 px-3 py-3 rounded-lg bg-brownish-50 dark:bg-brownish-900/60 border border-brownish-200 dark:border-brownish-700"
        v-if="!editExpire"
      >
        <div class="min-w-0">
          <div class="text-xs uppercase tracking-wider text-brownish-300">Expires</div>
          <div class="text-sm truncate">{{ linkExpiresAt || 'No expiry set' }}</div>
        </div>
        <div class="flex gap-1.5 shrink-0">
          <BaseButton
            title="Remove expiry"
            :icon="mdiDelete"
            color="dark"
            small
            rounded-full
            :disabled="!expiresAt || loadingExpire || readOnly"
            @click.prevent="updateExpiry(undefined)"
          />
          <BaseButton
            title="Edit expiry"
            :icon="mdiPencil"
            color="dark"
            small
            rounded-full
            :disabled="loadingExpire || readOnly"
            @click.prevent="editExpire = true"
          />
        </div>
      </div>
      <div
        v-else
        class="flex flex-col sm:flex-row sm:items-end gap-2 px-3 py-3 rounded-lg bg-brownish-50 dark:bg-brownish-900/60 border border-brownish-200 dark:border-brownish-700"
      >
        <div class="flex-1 min-w-0">
          <AppDateTime
            v-model="expiresAt"
            name="expires-at"
            :disabled="loadingExpire || readOnly"
            :min="new Date()"
          />
        </div>
        <div class="flex justify-end gap-1.5">
          <BaseButton
            title="Save changes"
            :icon="mdiCheck"
            color="dark"
            small
            rounded-full
            :disabled="!expiresAt || loadingExpire || readOnly"
            @click.prevent="updateExpiry(expiresAt)"
          />
          <BaseButton
            title="Remove expiry"
            :icon="mdiDelete"
            color="dark"
            small
            rounded-full
            :disabled="!expiresAt || loadingExpire || readOnly"
            @click.prevent="updateExpiry(undefined)"
          />
          <BaseButton
            title="Cancel editing"
            :icon="mdiClose"
            color="dark"
            small
            rounded-full
            :disabled="loadingExpire"
            @click.prevent="editExpire = false"
          />
        </div>
      </div>
    </div>
    <div v-else class="space-y-4 px-3 py-4 rounded-lg bg-brownish-50 dark:bg-brownish-900/60 border border-brownish-200 dark:border-brownish-700">
      <p class="text-sm text-brownish-700 dark:text-brownish-200">
        This file doesn't have a public link yet. Create one to share it with anyone over a URL.
      </p>
      <BaseButton
        title="Create link"
        label="Create link"
        color="info"
        small
        data-testid="sharing-link-create"
        :disabled="readOnly"
        @click.prevent="create"
      />
    </div>
  </div>
  <PuppyLoader v-else />
</template>
