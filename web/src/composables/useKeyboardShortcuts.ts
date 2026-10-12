import { onMounted, onUnmounted } from 'vue'

export interface KeyboardShortcut {
  /** `KeyboardEvent.key` to match, compared case-insensitively ('a', '/', 'Escape'). */
  key: string
  /** Require Ctrl, or Cmd on macOS. */
  mod?: boolean
  /** Ignored for symbol keys like '+' or '/': the symbol already says whether
   *  Shift was needed to type it, and that differs between keyboard layouts. */
  shift?: boolean
  alt?: boolean
  /** Also fire while typing in a text field. Off by default so the field keeps its own keys. */
  inEditable?: boolean
  /** Also fire while focus is inside a dialog. Off by default so a page shortcut
   *  doesn't reach through an open modal to the view behind it. */
  inDialog?: boolean
  /** Checked on every press; returning false lets the event through untouched. */
  when?: () => boolean
  /** Suppress the browser's own handling of the key. On by default. */
  preventDefault?: boolean
  handler: (event: KeyboardEvent) => void
}

// Clicking a checkbox's label focuses the (visually hidden) input, so treating
// every input as editable would swallow Del/Ctrl+A right after ticking rows.
const NON_TEXT_INPUTS = [
  'checkbox',
  'radio',
  'button',
  'submit',
  'reset',
  'file',
  'image',
  'color',
  'range'
]
const EDITABLE = [
  `input${NON_TEXT_INPUTS.map((type) => `:not([type="${type}"])`).join('')}`,
  'textarea',
  'select',
  '[contenteditable=""]',
  '[contenteditable="true"]'
].join(', ')

const isSymbol = (key: string) => key.length === 1 && key.toLowerCase() === key.toUpperCase()

const matches = (shortcut: KeyboardShortcut, event: KeyboardEvent) =>
  event.key.toLowerCase() === shortcut.key.toLowerCase() &&
  (event.ctrlKey || event.metaKey) === !!shortcut.mod &&
  (isSymbol(shortcut.key) || event.shiftKey === !!shortcut.shift) &&
  event.altKey === !!shortcut.alt

/**
 * Binds `shortcuts` on the window for as long as the calling component is
 * mounted. The first matching shortcut wins; anything unmatched passes through.
 */
export function useKeyboardShortcuts(shortcuts: KeyboardShortcut[]) {
  const onKeydown = (event: KeyboardEvent) => {
    const target = event.target instanceof Element ? event.target : null
    const inEditable = () =>
      !!target?.closest(EDITABLE) || !!(target as HTMLElement | null)?.isContentEditable
    const inDialog = () => !!target?.closest('[role="dialog"]')

    const shortcut = shortcuts.find(
      (s) =>
        matches(s, event) &&
        (s.inEditable || !inEditable()) &&
        (s.inDialog || !inDialog()) &&
        (s.when?.() ?? true)
    )
    if (!shortcut) return

    if (shortcut.preventDefault ?? true) event.preventDefault()
    shortcut.handler(event)
  }

  onMounted(() => window.addEventListener('keydown', onKeydown))
  onUnmounted(() => window.removeEventListener('keydown', onKeydown))
}
