import { ref } from 'vue'
import { saveFileContent, SaveConflictError } from '!/storage/save'
import type { AppFile } from 'types'

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error' | 'conflict'

export function useMarkdownSave() {
  const isDirty = ref(false)
  const isSaving = ref(false)
  const saveStatus = ref<SaveStatus>('idle')
  /**
   * The content the user tried to save when the server responded 409.
   * Held so the conflict-resolution UI can re-issue the save with
   * `force = true` against exactly the same payload.
   */
  const conflictedContent = ref<string | null>(null)
  let lastSavedContent = ''
  let autoSaveTimer: ReturnType<typeof setTimeout> | null = null

  function setLastSaved(content: string) {
    lastSavedContent = content
  }

  function getLastSaved() {
    return lastSavedContent
  }

  async function save(file: AppFile | undefined, content: string, force = false) {
    if (!isDirty.value && !force) return
    if (isSaving.value) return
    if (!file?.key) return

    isSaving.value = true
    saveStatus.value = 'saving'

    try {
      await saveFileContent(file, content, force)
      lastSavedContent = content
      isDirty.value = false
      conflictedContent.value = null
      saveStatus.value = 'saved'

      setTimeout(() => {
        if (saveStatus.value === 'saved') saveStatus.value = 'idle'
      }, 3000)
    } catch (err) {
      if (err instanceof SaveConflictError) {
        // Hold onto the content that didn't make it through so the UI
        // can retry verbatim once the user opts to discard the other
        // edit. The dirty flag stays set — they haven't saved yet.
        conflictedContent.value = content
        saveStatus.value = 'conflict'
      } else {
        console.error('Failed to save markdown:', err)
        saveStatus.value = 'error'
        setTimeout(() => {
          if (saveStatus.value === 'error') saveStatus.value = 'idle'
        }, 5000)
      }
    } finally {
      isSaving.value = false
    }
  }

  /**
   * Re-issue the conflicted save with `force = true`. Called from the
   * conflict prompt when the user picks "discard remote and overwrite".
   */
  async function resolveConflict(file: AppFile | undefined) {
    const content = conflictedContent.value
    if (content === null) return
    await save(file, content, true)
  }

  /**
   * User picked "keep remote, drop my changes" on the conflict prompt.
   * Wipe the conflicted-content marker and the dirty flag — the
   * remote's pending edit will eventually finalize and the editor will
   * reload from it next time the file opens.
   */
  function discardConflict() {
    conflictedContent.value = null
    saveStatus.value = 'idle'
    isDirty.value = false
  }

  function resetAutoSaveTimer(isEditable: boolean, doSave: () => void) {
    if (autoSaveTimer) clearTimeout(autoSaveTimer)
    if (!isEditable) return
    // Block auto-save while a conflict is unresolved so we don't
    // hammer the server with 409s every 5 seconds.
    if (saveStatus.value === 'conflict') return

    autoSaveTimer = setTimeout(() => {
      if (isDirty.value) doSave()
    }, 5000)
  }

  function clearAutoSaveTimer() {
    if (autoSaveTimer) clearTimeout(autoSaveTimer)
  }

  function markDirty(content: string) {
    isDirty.value = content !== lastSavedContent
  }

  return {
    isDirty,
    isSaving,
    saveStatus,
    conflictedContent,
    save,
    resolveConflict,
    discardConflict,
    resetAutoSaveTimer,
    clearAutoSaveTimer,
    markDirty,
    setLastSaved,
    getLastSaved
  }
}
