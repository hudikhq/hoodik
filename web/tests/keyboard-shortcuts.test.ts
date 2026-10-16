import { afterEach, describe, expect, it, vi } from 'vitest'
import { mount, type VueWrapper } from '@vue/test-utils'
import { defineComponent, h } from 'vue'
import { useKeyboardShortcuts, type KeyboardShortcut } from '../src/composables/useKeyboardShortcuts'

let wrapper: VueWrapper | null = null

const bind = (shortcuts: KeyboardShortcut[], html = '') => {
  wrapper = mount(
    defineComponent({
      setup: () => {
        useKeyboardShortcuts(shortcuts)
        return () => h('div', { innerHTML: html })
      }
    }),
    { attachTo: document.body }
  )
}

const press = (init: KeyboardEventInit, target: EventTarget = window) => {
  const event = new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init })
  target.dispatchEvent(event)
  return event
}

describe('useKeyboardShortcuts', () => {
  afterEach(() => {
    wrapper?.unmount()
    wrapper = null
  })

  it('fires on Ctrl or Cmd and suppresses the browser default', () => {
    const handler = vi.fn()
    bind([{ key: 'a', mod: true, handler }])

    expect(press({ key: 'a', ctrlKey: true }).defaultPrevented).toBe(true)
    expect(press({ key: 'A', metaKey: true }).defaultPrevented).toBe(true)
    expect(handler).toHaveBeenCalledTimes(2)
  })

  it('requires the exact modifiers', () => {
    const handler = vi.fn()
    bind([{ key: 'a', mod: true, handler }])

    expect(press({ key: 'a' }).defaultPrevented).toBe(false)
    press({ key: 'a', ctrlKey: true, shiftKey: true })
    press({ key: 'a', ctrlKey: true, altKey: true })
    expect(handler).not.toHaveBeenCalled()
  })

  it('lets text fields and dialogs keep their own keys unless opted in', () => {
    const handler = vi.fn()
    const optedIn = vi.fn()
    bind(
      [
        { key: 'a', mod: true, handler },
        { key: 'k', mod: true, inEditable: true, inDialog: true, handler: optedIn }
      ],
      '<input id="field"><div role="dialog"><button id="in-dialog"></button></div>'
    )
    const field = document.getElementById('field')!
    const inDialog = document.getElementById('in-dialog')!

    expect(press({ key: 'a', ctrlKey: true }, field).defaultPrevented).toBe(false)
    expect(press({ key: 'a', ctrlKey: true }, inDialog).defaultPrevented).toBe(false)
    expect(handler).not.toHaveBeenCalled()

    press({ key: 'k', ctrlKey: true }, field)
    press({ key: 'k', ctrlKey: true }, inDialog)
    expect(optedIn).toHaveBeenCalledTimes(2)
  })

  it('still fires from a focused checkbox or button', () => {
    const handler = vi.fn()
    bind(
      [{ key: 'Delete', handler }],
      '<input id="check" type="checkbox"><input id="radio" type="radio"><button id="btn"></button>'
    )

    for (const id of ['check', 'radio', 'btn']) {
      expect(press({ key: 'Delete' }, document.getElementById(id)!).defaultPrevented).toBe(true)
    }
    expect(handler).toHaveBeenCalledTimes(3)
  })

  it('treats typed inputs, textareas and contenteditable as text fields', () => {
    const handler = vi.fn()
    bind(
      [{ key: 'Delete', handler }],
      '<input id="text"><input id="number" type="number"><textarea id="area"></textarea>' +
        '<div id="rich" contenteditable="true"></div>'
    )

    for (const id of ['text', 'number', 'area', 'rich']) {
      expect(press({ key: 'Delete' }, document.getElementById(id)!).defaultPrevented).toBe(false)
    }
    expect(handler).not.toHaveBeenCalled()
  })

  it('passes the event through when `when` says no', () => {
    const handler = vi.fn()
    let enabled = false
    bind([{ key: 'a', mod: true, when: () => enabled, handler }])

    expect(press({ key: 'a', ctrlKey: true }).defaultPrevented).toBe(false)
    enabled = true
    expect(press({ key: 'a', ctrlKey: true }).defaultPrevented).toBe(true)
    expect(handler).toHaveBeenCalledTimes(1)
  })

  it('unbinds when the component unmounts', () => {
    const handler = vi.fn()
    bind([{ key: 'a', mod: true, handler }])
    wrapper!.unmount()
    wrapper = null

    press({ key: 'a', ctrlKey: true })
    expect(handler).not.toHaveBeenCalled()
  })
})
