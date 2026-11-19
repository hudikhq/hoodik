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
import SpinnerIcon from '@/components/ui/SpinnerIcon.vue'
import { formatSize } from '!/index'

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

/** True while the file is fetched and decrypted before the browser save starts. */
const downloading = ref(false)
const downloadedBytes = ref(0)
const downloadError = ref<string>()

const downloadPercent = computed(() => {
  const total = link.value?.file_size
  if (!total) return 0
  return Math.min(100, Math.floor((downloadedBytes.value / total) * 100))
})

/**
 * Client-side download + decrypt for public link (server sends only ciphertext).
 *
 * The whole file is fetched and decrypted before the browser save dialog
 * appears, which takes a while for large files. Without feedback, visitors
 * assume nothing happens and click again, starting parallel downloads.
 */
const download = async () => {
  if (!link.value?.link_key_hex || downloading.value) return

  downloading.value = true
  downloadedBytes.value = 0
  downloadError.value = undefined

  try {
    await props.Links.formDownload(link.value.id, link.value.link_key_hex, (bytes) => {
      downloadedBytes.value = bytes
    })
  } catch (e) {
    const error = e as ErrorResponse<unknown>
    downloadError.value = error?.description || error?.message || String(e)
  } finally {
    downloading.value = false
  }
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
    :downloading="downloading"
    @download="download"
    @details="details"
  >
    <div
      v-if="downloading"
      class="absolute inset-0 z-20 flex items-center justify-center bg-brownish-950/80"
      role="status"
      aria-live="polite"
      data-testid="link-download-progress"
    >
      <div class="flex flex-col items-center max-w-sm px-6 text-center space-y-3">
        <SpinnerIcon :size="48" />
        <p class="text-lg">{{ $t('links.view.downloadPreparing') }}</p>
        <p class="text-sm text-brownish-300">{{ $t('links.view.downloadPreparingHint') }}</p>
        <template v-if="link?.file_size">
          <div class="w-64 h-2 rounded bg-brownish-800 overflow-hidden">
            <div class="h-full bg-redish-500 transition-all" :style="{ width: `${downloadPercent}%` }" />
          </div>
          <p class="text-sm tabular-nums">
            {{ formatSize(downloadedBytes) }} / {{ formatSize(link.file_size) }} ({{ downloadPercent }} %)
          </p>
        </template>
      </div>
    </div>
    <div
      v-if="downloadError"
      class="absolute bottom-8 z-20 px-4 text-center text-redish-300 dark:text-redish-100"
      role="alert"
    >
      {{ $t('links.view.downloadFailed', { error: downloadError }) }}
    </div>
    <div class="absolute bottom-0" v-if="linkExpiresAt">
      <span v-if="!isExpired">{{ $t('links.view.willExpire', { date: linkExpiresAt }) }}</span>
      <span v-else class="text-redish-300 dark:text-redish-100">{{ $t('links.view.hasExpired', { date: linkExpiresAt }) }}</span>
    </div>
  </PreviewView>
  <LinkUnavailableInner v-else-if="linkUnavailable" />
  <EnterKeyInner v-else :unlockingError="unlockError" @unlock="unlock" />
</template>
