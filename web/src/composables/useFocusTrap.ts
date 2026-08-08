import { watch, nextTick, onUnmounted } from 'vue'

interface ReadonlyRef<T> {
  readonly value: T
}

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), ' +
  'textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'

/**
 * Keeps keyboard focus inside `container` while `active` is true.
 *
 * On activation focus moves to the first focusable element (or the container
 * itself), Tab and Shift+Tab wrap at the edges, and on deactivation focus
 * returns to whatever element had it before the trap opened.
 */
export function useFocusTrap(container: ReadonlyRef<HTMLElement | null>, active: ReadonlyRef<boolean>) {
  let previouslyFocused: HTMLElement | null = null

  const focusables = (): HTMLElement[] => {
    if (!container.value) return []
    return Array.from(container.value.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
      (el) => el.offsetParent !== null
    )
  }

  const onKeydown = (event: KeyboardEvent) => {
    if (event.key !== 'Tab' || !container.value) return

    const items = focusables()
    if (!items.length) {
      event.preventDefault()
      container.value.focus()
      return
    }

    const first = items[0]
    const last = items[items.length - 1]
    const current = document.activeElement as HTMLElement | null
    const inside = current && container.value.contains(current)

    if (event.shiftKey) {
      if (!inside || current === first) {
        event.preventDefault()
        last.focus()
      }
    } else if (!inside || current === last) {
      event.preventDefault()
      first.focus()
    }
  }

  const activate = async () => {
    previouslyFocused = document.activeElement as HTMLElement | null
    document.addEventListener('keydown', onKeydown)
    await nextTick()
    const items = focusables()
    if (items.length) {
      items[0].focus()
    } else {
      container.value?.focus()
    }
  }

  const deactivate = () => {
    document.removeEventListener('keydown', onKeydown)
    previouslyFocused?.focus()
    previouslyFocused = null
  }

  watch(
    () => active.value,
    (value) => (value ? activate() : deactivate()),
    { immediate: true }
  )

  // Most callers guard the dialog with `v-if`, so closing unmounts it before
  // the watcher can run deactivate() — without this, focus is left on <body>
  // and the next Tab restarts from the top of the document.
  onUnmounted(deactivate)
}
