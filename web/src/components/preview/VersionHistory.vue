<script setup lang="ts">
import { ref, watch } from 'vue'
import {
  mdiClose,
  mdiEye,
  mdiRestore,
  mdiContentDuplicate,
  mdiTrashCan,
  mdiDeleteSweep,
  mdiAlertCircleOutline
} from '@mdi/js'
import { MilkdownProvider } from '@milkdown/vue'
import { useI18n } from 'vue-i18n'
import MilkdownEditorInner from '@/components/editor/MilkdownEditorInner.vue'
import BaseButton from '@/components/ui/BaseButton.vue'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import CardBox from '@/components/ui/CardBox.vue'
import CardBoxComponentTitle from '@/components/ui/CardBoxComponentTitle.vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import OverlayLayer from '@/components/ui/OverlayLayer.vue'
import * as cryptfns from '!/cryptfns'
import * as versions from '!/storage/versions'
import type { ForkRequest } from '!/storage/versions'
import { formatPrettyDate, humanizeError } from '!/index'
import type { AppFile, FileVersion, KeyPair } from 'types'

import '@/components/editor/markdown-editor.css'

const props = defineProps<{
  file: AppFile
  keypair: KeyPair
}>()

const { t } = useI18n()

const emit = defineEmits<{
  (event: 'close'): void
  /** A version was just restored — host should reload the editor content. */
  (event: 'restored', file: AppFile): void
  /** A new note was forked off — host should navigate to it. */
  (event: 'forked', file: AppFile): void
  /** Active history list changed (delete/purge) — only used for refresh hints. */
  (event: 'changed'): void
}>()

const list = ref<FileVersion[]>([])
const loading = ref(false)
const loadError = ref<string | null>(null)
const busyVersion = ref<number | null>(null)
const previewVersion = ref<FileVersion | null>(null)
const previewBytes = ref<string | null>(null)
const previewError = ref<string | null>(null)
// A failed restore/fork/delete must not replace the list — the user needs
// to still see their versions while reading why the action failed.
const actionError = ref<string | null>(null)
const confirmingDelete = ref<FileVersion | null>(null)
const confirmingPurgeAll = ref(false)
const confirmingRestore = ref<FileVersion | null>(null)

async function load() {
  loading.value = true
  loadError.value = null
  try {
    list.value = await versions.list(props.file.id)
  } catch (err) {
    loadError.value = humanizeError(err)
  } finally {
    loading.value = false
  }
}

watch(() => props.file.id, load, { immediate: true })

function authorLabel(v: FileVersion): string {
  if (v.is_anonymous) return t('preview.versionHistory.authorAnonymous')
  if (v.user_id === props.file.user_id) return t('preview.versionHistory.authorYou')
  return t('preview.versionHistory.authorOther')
}

async function decryptVersionBytes(v: FileVersion): Promise<Uint8Array> {
  if (!props.file.key) throw new Error('File key unavailable')

  // Fetch every chunk and concatenate. Versioned downloads use the
  // owner's session cookie — no transfer token needed because history
  // access is owner-only. Chunks land in parallel; order comes from the
  // index, not arrival.
  const cipher = props.file.cipher
  const key = props.file.key
  const buffers = await Promise.all(
    [...new Array(v.chunks)].map(async (_, i) => {
      const encrypted = await versions.downloadChunk(props.file.id, v.version, i)
      return cryptfns.cipher.decrypt(cipher, encrypted, key, i)
    })
  )
  const total = buffers.reduce((sum, b) => sum + b.length, 0)
  const out = new Uint8Array(total)
  let offset = 0
  for (const b of buffers) {
    out.set(b, offset)
    offset += b.length
  }
  return out
}

async function openPreview(v: FileVersion) {
  previewVersion.value = v
  previewBytes.value = null
  previewError.value = null
  try {
    const bytes = await decryptVersionBytes(v)
    previewBytes.value = new TextDecoder().decode(bytes)
  } catch (err) {
    previewError.value = humanizeError(err)
  }
}

function closePreview() {
  previewVersion.value = null
  previewBytes.value = null
  previewError.value = null
}

function askRestore(v: FileVersion) {
  confirmingRestore.value = v
}

async function restore() {
  const v = confirmingRestore.value
  if (!v) return
  busyVersion.value = v.version
  actionError.value = null
  try {
    const updated = await versions.restore(props.file.id, v.version)
    emit('restored', updated)
    confirmingRestore.value = null
    await load()
  } catch (err) {
    actionError.value = humanizeError(err)
  } finally {
    busyVersion.value = null
  }
}

async function forkAsNew(v: FileVersion) {
  if (!props.file.key) {
    actionError.value = t('preview.versionHistory.keyUnavailable')
    return
  }
  busyVersion.value = v.version
  actionError.value = null
  try {
    const stamp = formatPrettyDate(v.created_at)
    const baseName = props.file.name.replace(/\.md$/i, '')
    const newName = `${baseName} (restored ${stamp}).md`

    const cipher = props.file.cipher
    const encryptedName = await cryptfns.cipher.encryptString(cipher, newName, props.file.key)

    // New file is owned by the same user and shares the source's
    // symmetric key (chunks are server-copied verbatim), so the
    // existing RSA-wrapped encrypted_key is reusable as-is.
    // This fork reuses the source's symmetric key verbatim (the comment
    // above), so its file scope is keyed on that same key.
    const rootKey = cryptfns.searchRootKey(props.keypair)
    const fileKey = cryptfns.searchFileKey(props.file.key)

    const payload: ForkRequest = {
      name_hash: cryptfns.searchTag(rootKey, newName),
      encrypted_name: encryptedName,
      encrypted_key: props.file.encrypted_key,
      mime: 'text/markdown',
      cipher,
      editable: true,
      file_id: props.file.file_id ?? undefined,
      search_tokens_root: cryptfns.searchTags(rootKey, newName.toLowerCase()),
      search_tokens_file: cryptfns.searchTags(fileKey, newName.toLowerCase())
    }

    const newFile = await versions.fork(props.file.id, v.version, payload)
    emit('forked', newFile)
  } catch (err) {
    actionError.value = humanizeError(err)
  } finally {
    busyVersion.value = null
  }
}

function askDelete(v: FileVersion) {
  confirmingDelete.value = v
}

async function confirmDelete() {
  const v = confirmingDelete.value
  if (!v) return
  busyVersion.value = v.version
  actionError.value = null
  try {
    await versions.remove(props.file.id, v.version)
    confirmingDelete.value = null
    await load()
    emit('changed')
  } catch (err) {
    actionError.value = humanizeError(err)
  } finally {
    busyVersion.value = null
  }
}

async function purgeAll() {
  actionError.value = null
  try {
    await versions.purgeAll(props.file.id)
    confirmingPurgeAll.value = false
    await load()
    emit('changed')
  } catch (err) {
    actionError.value = humanizeError(err)
  }
}
</script>

<template>
  <aside class="vh-panel">
    <header class="vh-header">
      <h3 class="vh-title">{{ $t('preview.versionHistory.title') }}</h3>
      <BaseButton color="dark" :icon="mdiClose" xs :title="$t('common.close')" name="vh-close" @click="emit('close')" />
    </header>

    <div v-if="actionError" class="vh-error" role="alert">
      <BaseIcon :path="mdiAlertCircleOutline" :size="14" />
      {{ actionError }}
    </div>

    <div v-if="loading" class="vh-empty">{{ $t('preview.versionHistory.loading') }}</div>

    <div v-else-if="loadError" class="vh-error">
      <BaseIcon :path="mdiAlertCircleOutline" :size="14" />
      {{ loadError }}
    </div>

    <div v-else-if="!list.length" class="vh-empty">
      {{ $t('preview.versionHistory.empty') }}
    </div>

    <ul v-else class="vh-list">
      <li v-for="v in list" :key="v.id" class="vh-item">
        <div class="vh-item-head">
          <!-- eslint-disable-next-line @intlify/vue-i18n/no-raw-text -->
          <span class="vh-item-version">v{{ v.version }}</span>
          <span class="vh-item-date">{{ formatPrettyDate(v.created_at) }}</span>
        </div>
        <div class="vh-item-meta">
          <span>{{ authorLabel(v) }}</span>
          <span class="vh-dot">·</span>
          <span>{{ $t('preview.versionHistory.chunks', v.chunks) }}</span>
        </div>
        <div class="vh-item-actions">
          <BaseButton color="dark" :icon="mdiEye" xs :title="$t('files.actions.preview')" name="vh-preview" @click="openPreview(v)" />
          <BaseButton
            color="dark"
            :icon="mdiRestore"
            xs
            :title="$t('preview.versionHistory.restoreInPlace')"
            name="vh-restore"
            :disabled="busyVersion === v.version"
            @click="askRestore(v)"
          />
          <BaseButton
            color="dark"
            :icon="mdiContentDuplicate"
            xs
            :title="$t('preview.versionHistory.restoreAsNew')"
            name="vh-fork"
            :disabled="busyVersion === v.version"
            @click="forkAsNew(v)"
          />
          <BaseButton
            color="danger"
            :icon="mdiTrashCan"
            xs
            :title="$t('preview.versionHistory.deleteVersion')"
            name="vh-delete"
            :disabled="busyVersion === v.version"
            @click="askDelete(v)"
          />
        </div>
      </li>
    </ul>

    <footer v-if="list.length" class="vh-footer">
      <BaseButton
        color="danger"
        :icon="mdiDeleteSweep"
        xs
        :label="$t('preview.versionHistory.clearAll')"
        name="vh-purge-all"
        @click="confirmingPurgeAll = true"
      />
    </footer>

    <!-- Preview gets a custom overlay (not CardBoxModal) so it can be
         much wider than the standard 4/12 modal — markdown rendering
         needs the room to breathe, especially with tables and code. -->
    <OverlayLayer :visible="!!previewVersion" @overlay-click="closePreview">
      <CardBox
        v-show="!!previewVersion"
        class="vh-preview-card relative max-h-modal w-11/12 lg:w-5/6 xl:w-3/4 z-50"
        is-modal
      >
        <CardBoxComponentTitle :title="previewVersion ? $t('preview.versionHistory.previewTitle', { version: previewVersion.version }) : ''" />
        <div v-if="previewError" class="vh-error">{{ previewError }}</div>
        <div v-else-if="previewBytes !== null" class="vh-preview milkdown-wrapper">
          <!-- Reuse the live editor in read-only mode so a previewed
               version renders identically to what the user sees while
               editing — same theme, same node styles, same fonts. The
               fresh `:key` forces a re-mount when the user opens a
               different version so Milkdown loads the new content. -->
          <MilkdownProvider :key="previewVersion?.id">
            <MilkdownEditorInner :content="previewBytes" :editable="false" />
          </MilkdownProvider>
        </div>
        <div v-else class="vh-empty">{{ $t('preview.versionHistory.decrypting') }}</div>
        <template #footer>
          <BaseButton color="info" :label="$t('common.close')" @click="closePreview" />
        </template>
      </CardBox>
    </OverlayLayer>

    <CardBoxModal
      :model-value="!!confirmingRestore"
      :title="$t('preview.versionHistory.restoreTitle')"
      button="warning"
      :button-label="$t('preview.versionHistory.restoreConfirmLabel')"
      has-cancel
      @cancel="confirmingRestore = null"
      @confirm="restore"
    >
      <p v-if="confirmingRestore">
        {{ $t('preview.versionHistory.restoreBody', {
          version: confirmingRestore.version,
          date: formatPrettyDate(confirmingRestore.created_at)
        }) }}
      </p>
    </CardBoxModal>

    <CardBoxModal
      :model-value="!!confirmingDelete"
      :title="$t('preview.versionHistory.deleteTitle')"
      button="danger"
      :button-label="$t('preview.versionHistory.deleteConfirmLabel')"
      has-cancel
      @cancel="confirmingDelete = null"
      @confirm="confirmDelete"
    >
      <p v-if="confirmingDelete">
        {{ $t('preview.versionHistory.deleteBody', {
          version: confirmingDelete.version,
          date: formatPrettyDate(confirmingDelete.created_at)
        }) }}
      </p>
    </CardBoxModal>

    <CardBoxModal
      :model-value="confirmingPurgeAll"
      :title="$t('preview.versionHistory.clearAllTitle')"
      button="danger"
      :button-label="$t('preview.versionHistory.clearAllConfirmLabel')"
      has-cancel
      @cancel="confirmingPurgeAll = false"
      @confirm="purgeAll"
    >
      <p>
        {{ $t('preview.versionHistory.clearAllBody') }}
      </p>
    </CardBoxModal>
  </aside>
</template>

<style scoped>
.vh-panel {
  display: flex;
  flex-direction: column;
  width: 22rem;
  max-width: 100%;
  background: #FAFAF9;
  border-left: 1px solid rgba(0, 0, 0, 0.1);
  overflow: hidden;
}

.dark .vh-panel {
  background: #181818;
  border-left-color: rgba(255, 255, 255, 0.08);
}

.vh-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid rgba(0, 0, 0, 0.08);
}

.dark .vh-header { border-bottom-color: rgba(255, 255, 255, 0.08); }

.vh-title {
  font-size: 0.875rem;
  font-weight: 600;
  color: #333333;
  letter-spacing: 0.025em;
}

.dark .vh-title { color: #d4d4d4; }

.vh-list {
  flex: 1;
  overflow-y: auto;
  padding: 0.25rem 0;
}

.vh-item {
  padding: 0.6rem 1rem;
  border-bottom: 1px solid rgba(0, 0, 0, 0.05);
}

.dark .vh-item { border-bottom-color: rgba(255, 255, 255, 0.04); }

.vh-item-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.5rem;
}

.vh-item-version {
  font-size: 0.8125rem;
  font-weight: 600;
  color: #C76F2C;
}

.dark .vh-item-version { color: #EE9B5C; }

.vh-item-date {
  font-size: 0.75rem;
  color: #8A8A86;
}

.dark .vh-item-date { color: #707070; }

.vh-item-meta {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.7rem;
  color: #6B6B66;
  margin-top: 0.25rem;
}

.dark .vh-item-meta { color: #909090; }

.vh-dot { color: #B9B9B4; }
.dark .vh-dot { color: #555; }

.vh-item-actions {
  display: flex;
  gap: 0.25rem;
  margin-top: 0.5rem;
}

.vh-footer {
  padding: 0.75rem 1rem;
  border-top: 1px solid rgba(0, 0, 0, 0.08);
}

.dark .vh-footer { border-top-color: rgba(255, 255, 255, 0.08); }

.vh-empty {
  padding: 1.5rem 1rem;
  font-size: 0.8125rem;
  color: #6B6B66;
  text-align: center;
}

.dark .vh-empty { color: #909090; }

.vh-error {
  padding: 0.75rem 1rem;
  font-size: 0.8125rem;
  color: #A43144;
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.dark .vh-error { color: #ff8888; }

/* The modal itself caps at `calc(100vh - 160px)` (max-h-modal). Inside
   it we lose ~150px to the title, body padding, and footer button.
   Cap the preview body so the footer button stays on-screen — without
   this the body's min-height pushes the close button below the fold,
   which is what the user hit. */
.vh-preview {
  max-height: calc(100vh - 320px);
  overflow-y: auto;
  overflow-x: hidden;
  border: 1px solid rgba(0, 0, 0, 0.08);
  border-radius: 0.375rem;
}

.dark .vh-preview { border-color: rgba(255, 255, 255, 0.06); }

/* Match the editor's own scrollbar treatment so the right edge looks
   intentional instead of revealing the OS-default scrollbar over the
   canvas. The preview's background comes from .milkdown-wrapper, which
   already follows the theme. */
.vh-preview::-webkit-scrollbar { width: 10px; }
.vh-preview::-webkit-scrollbar-track { background: transparent; }
.vh-preview::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.18);
  border-radius: 5px;
}
.vh-preview::-webkit-scrollbar-thumb:hover { background: rgba(0, 0, 0, 0.3); }
.dark .vh-preview::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.12); }
.dark .vh-preview::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.2); }
</style>
