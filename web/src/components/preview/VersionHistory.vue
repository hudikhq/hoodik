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
import { formatPrettyDate } from '!/index'
import type { AppFile, FileVersion, KeyPair } from 'types'

import '@/components/editor/markdown-editor.css'

const props = defineProps<{
  file: AppFile
  keypair: KeyPair
}>()

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
const confirmingDelete = ref<FileVersion | null>(null)
const confirmingPurgeAll = ref(false)
const confirmingRestore = ref<FileVersion | null>(null)

async function load() {
  loading.value = true
  loadError.value = null
  try {
    list.value = await versions.list(props.file.id)
  } catch (err) {
    loadError.value = (err as Error).message
  } finally {
    loading.value = false
  }
}

watch(() => props.file.id, load, { immediate: true })

function authorLabel(v: FileVersion): string {
  if (v.is_anonymous) return 'Anonymous'
  if (v.user_id === props.file.user_id) return 'You'
  return 'Another collaborator'
}

async function decryptVersionBytes(v: FileVersion): Promise<Uint8Array> {
  if (!props.file.key) throw new Error('File key unavailable')

  // Fetch every chunk and concatenate. Versioned downloads use the
  // owner's session cookie — no transfer token needed because history
  // access is owner-only.
  const cipher = props.file.cipher
  const buffers: Uint8Array[] = []
  for (let i = 0; i < v.chunks; i++) {
    const encrypted = await versions.downloadChunk(props.file.id, v.version, i)
    const decrypted = await cryptfns.cipher.decrypt(cipher, encrypted, props.file.key)
    buffers.push(decrypted)
  }
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
    previewError.value = (err as Error).message
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
  try {
    const updated = await versions.restore(props.file.id, v.version)
    emit('restored', updated)
    confirmingRestore.value = null
    await load()
  } catch (err) {
    loadError.value = (err as Error).message
  } finally {
    busyVersion.value = null
  }
}

async function forkAsNew(v: FileVersion) {
  if (!props.file.key) {
    loadError.value = 'File key unavailable'
    return
  }
  busyVersion.value = v.version
  try {
    const stamp = formatPrettyDate(v.created_at)
    const baseName = props.file.name.replace(/\.md$/i, '')
    const newName = `${baseName} (restored ${stamp}).md`

    const cipher = props.file.cipher
    const encryptedName = await cryptfns.cipher.encryptString(cipher, newName, props.file.key)

    // New file is owned by the same user and shares the source's
    // symmetric key (chunks are server-copied verbatim), so the
    // existing RSA-wrapped encrypted_key is reusable as-is.
    const payload: ForkRequest = {
      name_hash: cryptfns.sha256.digest(newName),
      encrypted_name: encryptedName,
      encrypted_key: props.file.encrypted_key,
      mime: 'text/markdown',
      cipher,
      editable: true,
      file_id: props.file.file_id,
      search_tokens_hashed: cryptfns.stringToHashedTokens(newName.toLowerCase())
    }

    const newFile = await versions.fork(props.file.id, v.version, payload)
    emit('forked', newFile)
  } catch (err) {
    loadError.value = (err as Error).message
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
  try {
    await versions.remove(props.file.id, v.version)
    confirmingDelete.value = null
    await load()
    emit('changed')
  } catch (err) {
    loadError.value = (err as Error).message
  } finally {
    busyVersion.value = null
  }
}

async function purgeAll() {
  try {
    await versions.purgeAll(props.file.id)
    confirmingPurgeAll.value = false
    await load()
    emit('changed')
  } catch (err) {
    loadError.value = (err as Error).message
  }
}
</script>

<template>
  <aside class="vh-panel">
    <header class="vh-header">
      <h3 class="vh-title">Version history</h3>
      <BaseButton color="dark" :icon="mdiClose" xs title="Close" name="vh-close" @click="emit('close')" />
    </header>

    <div v-if="loading" class="vh-empty">Loading versions…</div>

    <div v-else-if="loadError" class="vh-error">
      <BaseIcon :path="mdiAlertCircleOutline" :size="14" />
      {{ loadError }}
    </div>

    <div v-else-if="!list.length" class="vh-empty">
      No history yet. Edit the note to start building one.
    </div>

    <ul v-else class="vh-list">
      <li v-for="v in list" :key="v.id" class="vh-item">
        <div class="vh-item-head">
          <span class="vh-item-version">v{{ v.version }}</span>
          <span class="vh-item-date">{{ formatPrettyDate(v.created_at) }}</span>
        </div>
        <div class="vh-item-meta">
          <span>{{ authorLabel(v) }}</span>
          <span class="vh-dot">·</span>
          <span>{{ v.chunks }} chunk{{ v.chunks === 1 ? '' : 's' }}</span>
        </div>
        <div class="vh-item-actions">
          <BaseButton color="dark" :icon="mdiEye" xs title="Preview" name="vh-preview" @click="openPreview(v)" />
          <BaseButton
            color="dark"
            :icon="mdiRestore"
            xs
            title="Restore in place"
            name="vh-restore"
            :disabled="busyVersion === v.version"
            @click="askRestore(v)"
          />
          <BaseButton
            color="dark"
            :icon="mdiContentDuplicate"
            xs
            title="Restore as new note"
            name="vh-fork"
            :disabled="busyVersion === v.version"
            @click="forkAsNew(v)"
          />
          <BaseButton
            color="danger"
            :icon="mdiTrashCan"
            xs
            title="Delete this version"
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
        label="Clear all history"
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
        class="vh-preview-card shadow-lg max-h-modal w-11/12 lg:w-5/6 xl:w-3/4 z-50"
        is-modal
      >
        <CardBoxComponentTitle :title="previewVersion ? `Preview v${previewVersion.version}` : ''" />
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
        <div v-else class="vh-empty">Decrypting…</div>
        <template #footer>
          <BaseButton color="info" label="Close" @click="closePreview" />
        </template>
      </CardBox>
    </OverlayLayer>

    <CardBoxModal
      :model-value="!!confirmingRestore"
      title="Restore this version?"
      button="warning"
      button-label="Yes, restore"
      has-cancel
      @cancel="confirmingRestore = null"
      @confirm="restore"
    >
      <p>
        This replaces the current content with v{{ confirmingRestore?.version }} from
        {{ confirmingRestore ? formatPrettyDate(confirmingRestore.created_at) : '' }}. Your current version stays
        in history, so you can undo the restore later.
      </p>
    </CardBoxModal>

    <CardBoxModal
      :model-value="!!confirmingDelete"
      title="Delete this version?"
      button="danger"
      button-label="Delete forever"
      has-cancel
      @cancel="confirmingDelete = null"
      @confirm="confirmDelete"
    >
      <p>
        v{{ confirmingDelete?.version }} from
        {{ confirmingDelete ? formatPrettyDate(confirmingDelete.created_at) : '' }} will be removed permanently.
        This cannot be undone.
      </p>
    </CardBoxModal>

    <CardBoxModal
      :model-value="confirmingPurgeAll"
      title="Clear all history?"
      button="danger"
      button-label="Delete all"
      has-cancel
      @cancel="confirmingPurgeAll = false"
      @confirm="purgeAll"
    >
      <p>
        Every historical version will be permanently deleted. The current note stays.
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
  background: #181818;
  border-left: 1px solid rgba(255, 255, 255, 0.08);
  overflow: hidden;
}

.vh-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}

.vh-title {
  font-size: 0.875rem;
  font-weight: 600;
  color: #d4d4d4;
  letter-spacing: 0.025em;
}

.vh-list {
  flex: 1;
  overflow-y: auto;
  padding: 0.25rem 0;
}

.vh-item {
  padding: 0.6rem 1rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.04);
}

.vh-item-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 0.5rem;
}

.vh-item-version {
  font-size: 0.8125rem;
  font-weight: 600;
  color: #EE9B5C;
}

.vh-item-date {
  font-size: 0.75rem;
  color: #707070;
}

.vh-item-meta {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 0.7rem;
  color: #909090;
  margin-top: 0.25rem;
}

.vh-dot { color: #555; }

.vh-item-actions {
  display: flex;
  gap: 0.25rem;
  margin-top: 0.5rem;
}

.vh-footer {
  padding: 0.75rem 1rem;
  border-top: 1px solid rgba(255, 255, 255, 0.08);
}

.vh-empty {
  padding: 1.5rem 1rem;
  font-size: 0.8125rem;
  color: #909090;
  text-align: center;
}

.vh-error {
  padding: 0.75rem 1rem;
  font-size: 0.8125rem;
  color: #ff8888;
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

/* The modal itself caps at `calc(100vh - 160px)` (max-h-modal). Inside
   it we lose ~150px to the title, body padding, and footer button.
   Cap the preview body so the footer button stays on-screen — without
   this the body's min-height pushes the close button below the fold,
   which is what the user hit. */
.vh-preview {
  max-height: calc(100vh - 320px);
  overflow-y: auto;
  overflow-x: hidden;
  background: #181818;
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 0.375rem;
}

/* Override Milkdown's wrapper-level background so the dark theme is
   uninterrupted edge-to-edge inside the modal. */
.vh-preview.milkdown-wrapper {
  background: #181818;
}

/* Match the editor's own scrollbar treatment so the right edge looks
   intentional instead of revealing the OS-default light scrollbar
   over the dark canvas. */
.vh-preview::-webkit-scrollbar { width: 10px; }
.vh-preview::-webkit-scrollbar-track { background: transparent; }
.vh-preview::-webkit-scrollbar-thumb {
  background: rgba(255, 255, 255, 0.12);
  border-radius: 5px;
}
.vh-preview::-webkit-scrollbar-thumb:hover {
  background: rgba(255, 255, 255, 0.2);
}
</style>
