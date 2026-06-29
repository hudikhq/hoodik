<script lang="ts" setup>
import type { AppLink, LinksStore } from 'types'
import EnterKeyInner from './EnterKeyInner.vue'
import LinkUnavailableInner from './LinkUnavailableInner.vue'
import { computed, ref } from 'vue'
import type { ErrorResponse } from '!/api'
import { LinkPreview } from '!/preview/link'
import PreviewView from '@/components/preview/PreviewView.vue'
import { formatPrettyDate } from '!/index'
import PreviewInfoModal from '@/components/links/modals/PreviewInfoModal.vue'
import { useTitle } from '@vueuse/core'

const props = defineProps<{
  Links: LinksStore
  id: string
  linkKeyHex?: string
}>()

const title = useTitle()
const infoLink = ref()
const unlockError = ref()
/** True when the backend has confirmed the link doesn't exist — revoked
 *  or never created. Distinct from a missing / wrong unlock key so the
 *  page can swap the unlock form for a friendly "no longer available"
 *  panel instead of repeatedly prompting for a key that won't help. */
const linkUnavailable = ref(false)
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
 * Load the binary data of the link from the backend.
 *
 * Two distinct failure modes split out here: a 404 from the metadata
 * endpoint means the link itself is gone (revoked or expired beyond
 * the server's grace window), so the page renders an unavailable
 * panel. Any other failure (wrong key, decrypt failure, network) keeps
 * the unlock form on screen with the error text — the caller can
 * retype the key without leaving the page.
 */
const load = async () => {
  if (!linkKeyHex.value) return

  try {
    link.value = await props.Links.get(props.id, linkKeyHex.value)

    title.value = `${link.value.name} -- ${window.defaultDocumentTitle}`
  } catch (e) {
    const error = e as ErrorResponse<unknown>
    if (error?.status === 404) {
      linkUnavailable.value = true
      unlockError.value = undefined
      return
    }
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
  <LinkUnavailableInner v-else-if="linkUnavailable" />
  <EnterKeyInner v-else :unlockingError="unlockError" @unlock="unlock" />
</template>
