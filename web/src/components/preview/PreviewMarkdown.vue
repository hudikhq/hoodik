<script setup lang="ts">
import { ref, watch, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { MilkdownProvider } from '@milkdown/vue'
import { mdiNotePlusOutline } from '@mdi/js'
import BaseIcon from '@/components/ui/BaseIcon.vue'
import CardBoxModal from '@/components/ui/CardBoxModal.vue'
import FolderPicker from '@/components/ui/FolderPicker.vue'
import MilkdownEditorInner from '@/components/editor/MilkdownEditorInner.vue'
import MarkdownToolbar from '@/components/editor/MarkdownToolbar.vue'
import MarkdownActions from '@/components/editor/MarkdownActions.vue'
import RenameModal from '@/components/files/modals/RenameModal.vue'
import DetailsModal from '@/components/files/modals/DetailsModal.vue'
import LinkModal from '@/components/links/modals/LinkModal.vue'
import VersionHistory from '@/components/preview/VersionHistory.vue'
import { useMarkdownSave } from '@/components/editor/composables/useMarkdownSave'
import { exportPdf } from '@/components/editor/composables/useMarkdownExport'
import { notification } from '!/index'
import * as meta from '!/storage/meta'
import { emitFileTreeChange } from '!/storage/events'
import { store as storageStore } from '!/storage'
import { store as cryptoStore } from '!/crypto'
import { store as downloadStore } from '!/storage/download'
import { store as linksStore } from '!/links'
import { FilePreview } from '!/preview/file'
import type { Preview } from '!/preview'
import type { AppFile } from 'types'
import { useRouter } from 'vue-router'

import '@/components/editor/markdown-editor.css'

const props = defineProps<{
  modelValue: Preview
  readonly?: boolean
}>()

const preview = computed(() => props.modelValue)
const isOwnedFile = computed(() => preview.value instanceof FilePreview)

const editableOverride = ref<boolean | undefined>(undefined)

const isEditable = computed(() => {
  const editable = editableOverride.value ?? preview.value?.editable ?? false
  return !props.readonly && editable
})

const showToolbar = computed(() => isEditable.value || isOwnedFile.value)

const viewToggleLabel = computed(() => {
  if (showRaw.value) return isEditable.value ? 'WYSIWYG editor' : 'Rendered view'
  return 'Raw markdown'
})

const canConvertToNote = computed(() => {
  return isOwnedFile.value && !isEditable.value && !props.readonly
})

const markdownContent = ref('')
const isLoaded = ref(false)
const showRaw = ref(false)
const isConverting = ref(false)

const editorRef = ref<InstanceType<typeof MilkdownEditorInner>>()
const toolbarRef = ref<InstanceType<typeof MarkdownToolbar>>()
const actionsRef = ref<InstanceType<typeof MarkdownActions>>()
const editorWrapperRef = ref<HTMLElement>()

const renameFile = ref<AppFile>()
const detailsFile = ref<AppFile>()
const linkFile = ref<AppFile>()
const confirmingDelete = ref(false)
const showMoveModal = ref(false)
const moveFolderId = ref<string | undefined>()
const moveFolderName = ref('Root')

const Storage = storageStore()
const Crypto = cryptoStore()
const Download = downloadStore()
const Links = linksStore()
const router = useRouter()

const {
  isDirty, isSaving, saveStatus,
  save: doSave, resetAutoSaveTimer, clearAutoSaveTimer,
  markDirty, setLastSaved,
  resolveConflict, discardConflict
} = useMarkdownSave()

const showHistory = ref(false)
function toggleHistory() { showHistory.value = !showHistory.value }

function ownedFile(): AppFile | undefined {
  if (preview.value instanceof FilePreview) return (preview.value as FilePreview).file
}

function save() {
  doSave(ownedFile(), markdownContent.value)
}

async function onResolveConflict() {
  await resolveConflict(ownedFile())
}

function onDiscardConflict() {
  discardConflict()
}

async function onVersionRestored(updated: AppFile) {
  // Restore flipped active_version server-side — hand the new metadata
  // to the preview so the cached content doesn't shadow it, then reload.
  if (preview.value instanceof FilePreview) {
    ;(preview.value as FilePreview).updateFile(updated)
  }
  await load()
}

function onVersionForked(forkedFile: AppFile) {
  // Same UX as creating a new note: drop the user into it.
  router.push({ name: 'notes', params: { id: forkedFile.id } })
}

async function convertToNote() {
  const file = ownedFile()
  if (!file || isConverting.value) return

  isConverting.value = true
  try {
    await meta.setEditable(Crypto.keypair, file.id, true)
    editableOverride.value = true
    isLoaded.value = false
    await nextTick()
    isLoaded.value = true
  } catch (err) {
    console.error('Failed to convert file to note:', err)
  } finally {
    isConverting.value = false
  }
}

function openRename() { renameFile.value = ownedFile() }
function openDetails() { detailsFile.value = ownedFile() }
function openLink() { linkFile.value = ownedFile() }

function downloadFile() {
  const file = ownedFile()
  if (file) Download.push(file)
}

function promptDelete() { confirmingDelete.value = true }

async function deleteNote() {
  const file = ownedFile()
  if (!file) return

  clearAutoSaveTimer()
  isDirty.value = false

  await Storage.remove(Crypto.keypair, file)
  emitFileTreeChange({ type: 'deleted', folderId: file.file_id || undefined })
  router.push({ name: 'notes' })
}

function openMove() {
  moveFolderId.value = undefined
  moveFolderName.value = 'Root'
  showMoveModal.value = true
}

async function confirmMove() {
  const file = ownedFile()
  if (!file) return

  await meta.moveMany({ ids: [file.id], file_id: moveFolderId.value })
  emitFileTreeChange({ type: 'moved', folderId: file.file_id || undefined, targetFolderId: moveFolderId.value })
  showMoveModal.value = false
}

function handleExportPdf() {
  exportPdf(editorWrapperRef.value, preview.value.name || 'document')
}

function runCommand(command: string, payload?: unknown) {
  editorRef.value?.runCommand(command, payload)
}

function onContentUpdate(newContent: string) {
  markdownContent.value = newContent
  markDirty(newContent)
  if (isDirty.value) resetAutoSaveTimer(isEditable.value, save)
}

async function load() {
  isLoaded.value = false
  isDirty.value = false
  saveStatus.value = 'idle'
  editableOverride.value = undefined

  const data = await props.modelValue.load()
  const decoder = new TextDecoder()
  markdownContent.value = decoder.decode(data)
  setLastSaved(markdownContent.value)

  isLoaded.value = true
}

watch(() => props.modelValue, load, { immediate: true })

function onBeforeUnload(e: BeforeUnloadEvent) {
  if (isDirty.value) {
    e.preventDefault()
    e.returnValue = ''
  }
}

function onWindowBlur() {
  if (isDirty.value && isEditable.value) save()
}

function onDocumentClick(e: MouseEvent) {
  toolbarRef.value?.closeDropdown(e)
  actionsRef.value?.closeDropdown(e)
}

// Mac users see "⌘"; everyone else sees "Ctrl". Stays simple — no need
// for the full Platform detection dance because the click handler
// accepts both modifiers anyway.
const linkModifierLabel =
  typeof navigator !== 'undefined' && navigator.platform?.toLowerCase().includes('mac')
    ? '⌘'
    : 'Ctrl'

let modifierHintShown = false

function onEditorClick(e: MouseEvent) {
  const anchor = (e.target as HTMLElement).closest('a')
  if (!anchor) return

  const href = anchor.getAttribute('href')
  if (!href) return

  if (isEditable.value && !e.metaKey && !e.ctrlKey) {
    // Click swallowed in edit mode — links are click-to-edit by
    // default. Surface the modifier hint once per session so users
    // discover the right gesture without having to read docs.
    if (!modifierHintShown) {
      modifierHintShown = true
      notification(
        `${linkModifierLabel}+click to open links`,
        'Plain click in edit mode lets you change the link target.',
        'info'
      )
    }
    return
  }

  if (href.startsWith('#')) {
    e.preventDefault()
    const targetId = href.slice(1)
    const targetEl = editorWrapperRef.value?.querySelector(`[id="${CSS.escape(targetId)}"]`)
    if (targetEl) targetEl.scrollIntoView({ behavior: 'smooth', block: 'start' })
    return
  }

  e.preventDefault()
  window.open(href, '_blank', 'noopener')
}

/// One-shot pass that stamps `title="Cmd+click to open"` on every
/// existing anchor inside the editor. We deliberately avoid a
/// MutationObserver here — Milkdown rebuilds chunks of its DOM on
/// every keystroke, and watching that fires the callback so often it
/// freezes the tab. The click-time notification (see onEditorClick)
/// covers anchors the user adds after the initial render.
function annotateExistingLinks() {
  const root = editorWrapperRef.value
  if (!root || !isEditable.value) return
  const tooltip = `${linkModifierLabel}+click to open`
  for (const a of Array.from(root.querySelectorAll('a'))) {
    if (a.getAttribute('title')) continue
    a.setAttribute('title', tooltip)
  }
}

onMounted(() => {
  window.addEventListener('beforeunload', onBeforeUnload)
  window.addEventListener('blur', onWindowBlur)
  window.addEventListener('click', onDocumentClick)
})

// Run the link-title pass once after the editor finishes its first
// render. Cheap, no ongoing observers — much safer than watching
// Milkdown's mutations.
watch(
  () => isLoaded.value && isEditable.value && !showRaw.value,
  async (active) => {
    if (active) {
      await nextTick()
      annotateExistingLinks()
    }
  },
  { immediate: true }
)

onUnmounted(() => {
  clearAutoSaveTimer()
  if (isDirty.value && isEditable.value) save()

  window.removeEventListener('beforeunload', onBeforeUnload)
  window.removeEventListener('blur', onWindowBlur)
  window.removeEventListener('click', onDocumentClick)
})

defineExpose({ exportPdf: handleExportPdf })
</script>

<template>
  <div v-if="isLoaded" class="flex flex-col w-full h-full overflow-hidden">
    <!-- Toolbar -->
    <div v-if="showToolbar" class="md-toolbar flex items-center gap-1 px-4 py-2 flex-shrink-0 flex-wrap">
      <MarkdownToolbar
        ref="toolbarRef"
        :editable="isEditable"
        :is-dirty="isDirty"
        :is-saving="isSaving"
        :save-status="saveStatus"
        :show-history-button="isOwnedFile"
        :is-history-open="showHistory"
        @command="runCommand"
        @save="save"
        @toggle-history="toggleHistory"
      />
      <MarkdownActions
        ref="actionsRef"
        :show-raw="showRaw"
        :view-toggle-label="viewToggleLabel"
        :can-convert-to-note="canConvertToNote"
        @details="openDetails"
        @rename="openRename"
        @download="downloadFile"
        @link="openLink"
        @export-pdf="handleExportPdf"
        @toggle-raw="showRaw = !showRaw"
        @convert="convertToNote"
        @move="openMove"
        @delete="promptDelete"
      />
    </div>

    <!-- Modals -->
    <RenameModal v-if="renameFile" v-model="renameFile" :Storage="Storage" :Crypto="Crypto" />
    <DetailsModal v-model="detailsFile" :kp="Crypto.keypair" />
    <LinkModal v-model="linkFile" :Storage="Storage" :Links="Links" :kp="Crypto.keypair" />

    <CardBoxModal
      :model-value="confirmingDelete"
      title="Delete note"
      button="danger"
      button-label="Yes, delete"
      has-cancel
      @cancel="confirmingDelete = false"
      @confirm="deleteNote"
    >
      Are you sure you want to delete '{{ preview.name }}'?
    </CardBoxModal>

    <CardBoxModal
      :model-value="showMoveModal"
      title="Move to folder"
      button="info"
      button-label="Move here"
      has-cancel
      @cancel="showMoveModal = false"
      @confirm="confirmMove"
    >
      <FolderPicker
        :keypair="Crypto.keypair"
        @navigate="({ id, name }) => { moveFolderId = id; moveFolderName = name }"
      />
      <p class="mt-2 text-xs text-brownish-400">
        Move '{{ preview.name }}' to <strong>{{ moveFolderName }}</strong>
      </p>
    </CardBoxModal>

    <!-- Convert to note banner -->
    <div v-if="canConvertToNote" class="md-convert-banner">
      <BaseIcon :path="mdiNotePlusOutline" :size="16" />
      <span>This markdown file is read-only.</span>
      <button class="md-convert-link" :disabled="isConverting" @click="convertToNote">
        {{ isConverting ? 'Converting...' : 'Convert to an editable note' }}
      </button>
    </div>

    <!-- Save-conflict prompt — fired when the server returns 409 on
         replaceContent because a previous save is still pending. -->
    <CardBoxModal
      :model-value="saveStatus === 'conflict'"
      title="Another save is in progress"
      button="warning"
      button-label="Discard remote and overwrite"
      has-cancel
      @cancel="onDiscardConflict"
      @confirm="onResolveConflict"
    >
      The server has an unfinished save for this note from another session.
      Choose <strong>Discard remote and overwrite</strong> to drop their pending edit and save your version,
      or <strong>Cancel</strong> to drop your local changes and let the other save finish.
    </CardBoxModal>

    <!-- Editor body + optional version-history sidebar. -->
    <div class="flex-1 flex overflow-hidden">
      <div class="flex-1 flex flex-col overflow-hidden">
        <!-- Raw markdown editor -->
        <div v-if="showRaw" class="flex-1 overflow-auto md-raw-wrapper">
          <textarea
            :value="markdownContent"
            class="md-raw-textarea"
            :readonly="!isEditable"
            spellcheck="false"
            @input="onContentUpdate(($event.target as HTMLTextAreaElement).value)"
          />
        </div>

        <!-- WYSIWYG editor -->
        <div v-else ref="editorWrapperRef" class="flex-1 overflow-auto milkdown-wrapper" @click="onEditorClick">
          <MilkdownProvider>
            <MilkdownEditorInner
              ref="editorRef"
              :content="markdownContent"
              :editable="isEditable"
              @update:content="onContentUpdate"
              @save="save"
            />
          </MilkdownProvider>
        </div>
      </div>

      <VersionHistory
        v-if="showHistory && ownedFile()"
        :file="ownedFile()!"
        :keypair="Crypto.keypair"
        @close="showHistory = false"
        @restored="onVersionRestored"
        @forked="onVersionForked"
      />
    </div>
  </div>
</template>

<style scoped>
.md-toolbar {
  background: #1a1a1a;
  border-bottom: 1px solid rgba(238, 132, 52, 0.15);
}

.md-convert-banner {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  background: rgba(139, 169, 224, 0.06);
  border-bottom: 1px solid rgba(139, 169, 224, 0.12);
  color: #8BA9E0;
  font-size: 0.8125rem;
  flex-shrink: 0;
}

.md-convert-link {
  color: #EE9B5C;
  font-weight: 500;
  text-decoration: underline;
  text-underline-offset: 2px;
  transition: color 150ms;
}

.md-convert-link:hover { color: #F2AC78; }
.md-convert-link:disabled { opacity: 0.5; cursor: wait; }

.md-raw-wrapper { background: #141414; }

.md-raw-textarea {
  width: 100%;
  height: 100%;
  min-height: 100%;
  max-width: 52rem;
  margin: 0 auto;
  display: block;
  padding: 2rem 2.5rem;
  background: transparent;
  color: #b0b0b0;
  font-family: 'SF Mono', 'Fira Code', 'JetBrains Mono', 'Cascadia Code', monospace;
  font-size: 0.875rem;
  line-height: 1.8;
  resize: none;
  outline: none;
  border: none;
  tab-size: 2;
}

.md-raw-wrapper::-webkit-scrollbar { width: 8px; }
.md-raw-wrapper::-webkit-scrollbar-track { background: transparent; }
.md-raw-wrapper::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.08); border-radius: 4px; }
.md-raw-wrapper::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.15); }
</style>
