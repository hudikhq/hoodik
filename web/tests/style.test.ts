import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { store as styleStore } from '../services/style'
import { lightModeKey } from '../src/config'

describe('style store dark mode', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  afterEach(() => {
    localStorage.removeItem(lightModeKey)
    document.documentElement.classList.remove('dark')
  })

  it('sets dark mode when passed true', () => {
    const style = styleStore()
    style.setDarkMode(true)

    expect(style.darkMode).toBe(true)
    expect(localStorage.getItem(lightModeKey)).toBe('0')
    expect(document.documentElement.classList.contains('dark')).toBe(true)
  })

  it('sets light mode when passed false instead of toggling', () => {
    const style = styleStore()
    style.setDarkMode(false)
    style.setDarkMode(false)

    expect(style.darkMode).toBe(false)
    expect(localStorage.getItem(lightModeKey)).toBe('1')
    expect(document.documentElement.classList.contains('dark')).toBe(false)
  })

  it('toggles when called without an argument', () => {
    const style = styleStore()
    style.setDarkMode(true)
    style.setDarkMode()

    expect(style.darkMode).toBe(false)
    expect(localStorage.getItem(lightModeKey)).toBe('1')

    style.setDarkMode()

    expect(style.darkMode).toBe(true)
    expect(localStorage.getItem(lightModeKey)).toBe('0')
  })
})
