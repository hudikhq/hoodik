import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { nextTick } from 'vue'

/**
 * useMarkdownSave is the composable that drives the status UI on the
 * editor ("idle" / "saving" / "saved" / "error" / "conflict"). It owns:
 *   - a re-entrancy guard (a save can't overlap itself)
 *   - dirty tracking against the last successfully-saved content
 *   - a 5-second auto-save debounce
 *   - conflict resolution (retry with force=true, or drop the draft)
 * Every IO dependency (saveFileContent) is mocked so we test state
 * transitions only, no network.
 */

vi.mock('!/storage/save', () => {
  class SaveConflictError extends Error {
    readonly fileId: string
    readonly originalContent: string
    constructor(fileId: string, originalContent: string) {
      super('another_edit_is_in_progress')
      this.name = 'SaveConflictError'
      this.fileId = fileId
      this.originalContent = originalContent
    }
  }
  return {
    saveFileContent: vi.fn(),
    SaveConflictError
  }
})

import { useMarkdownSave } from '../src/components/editor/composables/useMarkdownSave'
import { saveFileContent, SaveConflictError } from '!/storage/save'
import type { AppFile } from 'types'

function makeFile(overrides: Partial<AppFile> = {}): AppFile {
  return {
    id: 'file-1',
    user_id: 'u1',
    is_owner: true,
    name_hash: 'h',
    name: 'note.md',
    mime: 'text/markdown',
    size: 10,
    chunks: 1,
    encrypted_key: '',
    encrypted_name: '',
    cipher: 'aegis-128l',
    editable: true,
    active_version: 1,
    file_modified_at: 0,
    created_at: 0,
    is_new: false,
    key: new Uint8Array(32),
    ...overrides
  } as AppFile
}

const saveMock = saveFileContent as unknown as ReturnType<typeof vi.fn>

describe('useMarkdownSave — state machine', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.runOnlyPendingTimers()
    vi.useRealTimers()
  })

  it('UNIT: idle → saving → saved on a successful save', async () => {
    saveMock.mockResolvedValue(makeFile({ active_version: 2 }))

    const ms = useMarkdownSave()
    ms.setLastSaved('')
    ms.markDirty('hello')
    expect(ms.isDirty.value).toBe(true)
    expect(ms.saveStatus.value).toBe('idle')

    const savePromise = ms.save(makeFile(), 'hello')
    // Synchronously after kicking off: we flipped to saving
    expect(ms.saveStatus.value).toBe('saving')
    expect(ms.isSaving.value).toBe(true)

    await savePromise
    expect(ms.saveStatus.value).toBe('saved')
    expect(ms.isDirty.value).toBe(false)
    expect(ms.isSaving.value).toBe(false)
    expect(ms.getLastSaved()).toBe('hello')
  })

  it('UNIT: saved auto-clears back to idle after 3s', async () => {
    saveMock.mockResolvedValue(makeFile())
    const ms = useMarkdownSave()
    ms.markDirty('x')
    await ms.save(makeFile(), 'x')
    expect(ms.saveStatus.value).toBe('saved')

    vi.advanceTimersByTime(3000)
    await nextTick()
    expect(ms.saveStatus.value).toBe('idle')
  })

  it('UNIT: idle → saving → conflict on a 409 response', async () => {
    saveMock.mockRejectedValueOnce(new SaveConflictError('file-1', 'draft-content'))

    const ms = useMarkdownSave()
    ms.markDirty('draft-content')
    await ms.save(makeFile(), 'draft-content')

    expect(ms.saveStatus.value).toBe('conflict')
    expect(ms.conflictedContent.value).toBe('draft-content')
    expect(ms.isDirty.value).toBe(true) // nothing made it through, still dirty
  })

  it('UNIT: idle → saving → error on a non-conflict failure; auto-clears after 5s', async () => {
    saveMock.mockRejectedValueOnce(new Error('network down'))
    const errSpy = vi.spyOn(console, 'error').mockImplementation(() => void 0)

    const ms = useMarkdownSave()
    ms.markDirty('x')
    await ms.save(makeFile(), 'x')
    expect(ms.saveStatus.value).toBe('error')
    expect(errSpy).toHaveBeenCalled()

    vi.advanceTimersByTime(5000)
    await nextTick()
    expect(ms.saveStatus.value).toBe('idle')

    errSpy.mockRestore()
  })

  it('UNIT: save is a no-op when not dirty and force=false', async () => {
    const ms = useMarkdownSave()
    // Fresh composable: isDirty is false, no markDirty called.
    await ms.save(makeFile(), 'anything')
    expect(saveMock).not.toHaveBeenCalled()
    expect(ms.saveStatus.value).toBe('idle')
  })

  it('UNIT: save is a no-op without a file key', async () => {
    const ms = useMarkdownSave()
    ms.markDirty('dirty')
    await ms.save(makeFile({ key: undefined }), 'content')
    expect(saveMock).not.toHaveBeenCalled()
  })

  it('UNIT: a second save() while one is in flight is ignored', async () => {
    // Never-resolving save so we can observe the re-entrancy guard.
    let resolveFirst: ((v: AppFile) => void) | undefined
    saveMock.mockImplementationOnce(
      () =>
        new Promise<AppFile>((r) => {
          resolveFirst = r
        })
    )

    const ms = useMarkdownSave()
    ms.markDirty('x')
    const p1 = ms.save(makeFile(), 'x')
    expect(ms.isSaving.value).toBe(true)

    // While first save is still hanging, try again
    const p2 = ms.save(makeFile(), 'y')
    await p2
    expect(saveMock).toHaveBeenCalledTimes(1)

    // Resolve the first; cleanup
    resolveFirst?.(makeFile())
    await p1
  })
})

describe('useMarkdownSave — markDirty', () => {
  it('UNIT: isDirty flips true when content drifts from last saved', () => {
    const ms = useMarkdownSave()
    ms.setLastSaved('initial')
    expect(ms.isDirty.value).toBe(false)

    ms.markDirty('different')
    expect(ms.isDirty.value).toBe(true)
  })

  it('UNIT: isDirty flips back to false when content matches last saved', () => {
    const ms = useMarkdownSave()
    ms.setLastSaved('initial')
    ms.markDirty('different')
    expect(ms.isDirty.value).toBe(true)

    ms.markDirty('initial')
    expect(ms.isDirty.value).toBe(false)
  })
})

describe('useMarkdownSave — auto-save debounce', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.runOnlyPendingTimers()
    vi.useRealTimers()
  })

  it('UNIT: resetAutoSaveTimer fires doSave ~5s later when dirty', () => {
    const ms = useMarkdownSave()
    ms.markDirty('dirty content')

    const doSave = vi.fn()
    ms.resetAutoSaveTimer(true, doSave)

    vi.advanceTimersByTime(4999)
    expect(doSave).not.toHaveBeenCalled()

    vi.advanceTimersByTime(1)
    expect(doSave).toHaveBeenCalledTimes(1)
  })

  it('UNIT: resetAutoSaveTimer cancels the previous timer (the last reset wins)', () => {
    const ms = useMarkdownSave()
    ms.markDirty('x')
    const doSave = vi.fn()

    ms.resetAutoSaveTimer(true, doSave)
    vi.advanceTimersByTime(3000)
    // Another keystroke — reset the debounce
    ms.resetAutoSaveTimer(true, doSave)
    vi.advanceTimersByTime(3000)
    expect(doSave).not.toHaveBeenCalled()

    // Now let the second timer finish
    vi.advanceTimersByTime(2000)
    expect(doSave).toHaveBeenCalledTimes(1)
  })

  it('UNIT: resetAutoSaveTimer does NOT fire when not dirty', () => {
    const ms = useMarkdownSave()
    const doSave = vi.fn()
    // isDirty starts false; never called markDirty.
    ms.resetAutoSaveTimer(true, doSave)
    vi.advanceTimersByTime(10_000)
    expect(doSave).not.toHaveBeenCalled()
  })

  it('UNIT: resetAutoSaveTimer is a no-op for non-editable files', () => {
    const ms = useMarkdownSave()
    ms.markDirty('x')
    const doSave = vi.fn()

    ms.resetAutoSaveTimer(false, doSave)
    vi.advanceTimersByTime(10_000)
    expect(doSave).not.toHaveBeenCalled()
  })

  it('UNIT: auto-save is suppressed while in conflict state', async () => {
    saveMock.mockRejectedValueOnce(new SaveConflictError('file-1', 'draft'))

    const ms = useMarkdownSave()
    ms.markDirty('draft')
    await ms.save(makeFile(), 'draft')
    expect(ms.saveStatus.value).toBe('conflict')

    const doSave = vi.fn()
    ms.resetAutoSaveTimer(true, doSave)
    vi.advanceTimersByTime(10_000)
    expect(doSave).not.toHaveBeenCalled()
  })

  it('UNIT: clearAutoSaveTimer stops a pending debounce', () => {
    const ms = useMarkdownSave()
    ms.markDirty('x')
    const doSave = vi.fn()

    ms.resetAutoSaveTimer(true, doSave)
    vi.advanceTimersByTime(2000)
    ms.clearAutoSaveTimer()
    vi.advanceTimersByTime(10_000)
    expect(doSave).not.toHaveBeenCalled()
  })
})

describe('useMarkdownSave — conflict resolution', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('UNIT: resolveConflict re-saves the draft with force=true', async () => {
    saveMock.mockRejectedValueOnce(new SaveConflictError('file-1', 'my draft'))
    saveMock.mockResolvedValueOnce(makeFile({ active_version: 3 }))

    const ms = useMarkdownSave()
    ms.markDirty('my draft')
    await ms.save(makeFile(), 'my draft')
    expect(ms.saveStatus.value).toBe('conflict')

    await ms.resolveConflict(makeFile())

    expect(saveMock).toHaveBeenCalledTimes(2)
    const [, content, force] = saveMock.mock.calls[1]
    expect(content).toBe('my draft')
    expect(force).toBe(true)
    expect(ms.saveStatus.value).toBe('saved')
    expect(ms.conflictedContent.value).toBeNull()
  })

  it('UNIT: resolveConflict is a no-op when there is no conflicted content', async () => {
    const ms = useMarkdownSave()
    await ms.resolveConflict(makeFile())
    expect(saveMock).not.toHaveBeenCalled()
  })

  it('UNIT: discardConflict clears the conflict and resets state', async () => {
    saveMock.mockRejectedValueOnce(new SaveConflictError('file-1', 'draft'))
    const ms = useMarkdownSave()
    ms.markDirty('draft')
    await ms.save(makeFile(), 'draft')
    expect(ms.saveStatus.value).toBe('conflict')
    expect(ms.conflictedContent.value).toBe('draft')
    expect(ms.isDirty.value).toBe(true)

    ms.discardConflict()
    expect(ms.saveStatus.value).toBe('idle')
    expect(ms.conflictedContent.value).toBeNull()
    expect(ms.isDirty.value).toBe(false)
  })
})
