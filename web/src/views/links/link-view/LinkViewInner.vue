<script lang="ts" setup>
import type { AppLink, LinksStore } from 'types'
import LinkPreview from './LinkPreview.vue'
import EnterKeyInner from './EnterKeyInner.vue'
import { computed, ref } from 'vue'
import type { ErrorResponse } from '!/api'

const props = defineProps<{
  Links: LinksStore
  id: string
  linkKeyHex?: string
}>()

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
 * Set the link unlock key, and attempt loading again
 */
const unlock = async (value: string) => {
  linkKeyHex.value = value

  await load()
}

await load()
</script>
<template>
  <LinkPreview v-if="link" v-model="link" :Links="Links" />
  <EnterKeyInner v-else :unlockingError="unlockError" @unlock="unlock" />
</template>
