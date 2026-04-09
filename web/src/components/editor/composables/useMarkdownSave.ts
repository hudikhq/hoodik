import { ref } from 'vue'
import { saveFileContent } from '!/storage/save'
import type { AppFile } from 'types'

export type SaveStatus = 'idle' | 'saving' | 'saved' | 'error'

export function useMarkdownSave() {
  const isDirty = ref(false)
  const isSaving = ref(false)
  const saveStatus = ref<SaveStatus>('idle')
  let lastSavedContent = ''
  let autoSaveTimer: ReturnType<typeof setTimeout> | null = null

  function setLastSaved(content: string) {
    lastSavedContent = content
  }

  function getLastSaved() {
    return lastSavedContent
  }

  async function save(file: AppFile | undefined, content: string) {
    if (!isDirty.value || isSaving.value) return
    if (!file?.key) return

    isSaving.value = true
    saveStatus.value = 'saving'

    try {
      await saveFileContent(file, content)
      lastSavedContent = content
      isDirty.value = false
      saveStatus.value = 'saved'

      setTimeout(() => {
        if (saveStatus.value === 'saved') saveStatus.value = 'idle'
      }, 3000)
    } catch (err) {
      console.error('Failed to save markdown:', err)
      saveStatus.value = 'error'

      setTimeout(() => {
        if (saveStatus.value === 'error') saveStatus.value = 'idle'
      }, 5000)
    } finally {
      isSaving.value = false
    }
  }

  function resetAutoSaveTimer(isEditable: boolean, doSave: () => void) {
    if (autoSaveTimer) clearTimeout(autoSaveTimer)
    if (!isEditable) return

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
    save,
    resetAutoSaveTimer,
    clearAutoSaveTimer,
    markDirty,
    setLastSaved,
    getLastSaved,
  }
}
