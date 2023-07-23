<script lang="ts" setup>
import type { AppLink, LinksStore } from 'types'
import EnterKeyInner from './EnterKeyInner.vue'
import { computed, ref } from 'vue'
import type { ErrorResponse } from '!/api'
import { LinkPreview } from '!/preview/link'
import PreviewView from '@/components/preview/PreviewView.vue'
import { formatPrettyDate } from '!/index'
import PreviewInfoModal from '@/components/links/modals/PreviewInfoModal.vue'

const props = defineProps<{
  Links: LinksStore
  id: string
  linkKeyHex?: string
}>()

const infoLink = ref()
const unlockError = ref()
const typedLinkKeyHex = ref<string>()
const link = ref<AppLink>()

const linkKeyHex = computed({
  get: (): string | undefined => {
    if (props.linkKeyHex) {
      return props.linkKeyHex
    }

    return typedLinkKeyHex.value
  },
  set: (v: string | undefined): void => {
    typedLinkKeyHex.value = v
  }
})

const preview = computed(() => {
  if (!link.value) return

  return new LinkPreview(link.value)
})

const linkExpiresAt = computed(() => {
  return link.value?.expires_at ? formatPrettyDate(link.value?.expires_at) : null
})

const isExpired = computed(() => {
  const now = new Date().valueOf() / 1000

  return link.value?.expires_at && link.value?.expires_at < now
})

/**
 * Load the binary data of the link from the backend
 */
const load = async () => {
  if (!linkKeyHex.value) return

  try {
    link.value = await props.Links.get(props.id, linkKeyHex.value)
  } catch (e) {
    const error = e as ErrorResponse<unknown>
    unlockError.value = error.description || error.message
  }
}

/**
 * Start the download of a link through the
 * regular download process.
 */
const download = async () => {
  if (!link.value) return

  await props.Links.formDownload(link.value.id, link.value.link_key_hex)
}

/**
 * Open the details modal with verified signature
 */
const details = async () => {
  infoLink.value = link.value
}

/**
 * Set the link unlock key, and attempt loading again
 */
const unlock = async (value: string) => {
  linkKeyHex.value = value

  await load()
}

await load()
</script>
<template>
  <PreviewInfoModal v-model="infoLink" />

  <PreviewView
    v-if="preview"
    v-model="preview"
    :hideDelete="true"
    :hidePreviousAndNext="true"
    :hideClose="true"
    @download="download"
    @details="details"
  >
    <div class="absolute bottom-0" v-if="linkExpiresAt">
      <span v-if="!isExpired">This link will expire on {{ linkExpiresAt }}</span>
      <span v-else class="text-redish-300">This link has expired on {{ linkExpiresAt }}</span>
    </div>
  </PreviewView>
  <EnterKeyInner v-else :unlockingError="unlockError" @unlock="unlock" />
</template>
