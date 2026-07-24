import { describe, it, expect, afterEach } from 'vitest'
import { i18n, setLocale, detectLocale, translateErrorCode } from '../src/i18n'
import { localeKey } from '../src/config'
import { formatRelative, formatSize } from '../services'

afterEach(() => {
  localStorage.removeItem(localeKey)
  i18n.global.locale.value = 'en'
})

describe('locale detection', () => {
  it('falls back to english when nothing is stored', () => {
    expect(detectLocale()).toBe('en')
  })

  it('prefers the stored locale over the browser language', () => {
    localStorage[localeKey] = 'hr'
    expect(detectLocale()).toBe('hr')
  })

  it('ignores stored values outside the supported set', () => {
    localStorage[localeKey] = 'xx'
    expect(detectLocale()).toBe('en')
  })

  it('persists the choice and switches the active locale', () => {
    setLocale('de')
    expect(localStorage[localeKey]).toBe('de')
    expect(i18n.global.locale.value).toBe('de')
  })
})

describe('server error codes', () => {
  it('translates known codes', () => {
    expect(translateErrorCode('quota_exceeded')).toBe('Storage quota exceeded')
  })

  it('keeps the detail suffix of composite codes', () => {
    expect(translateErrorCode('invalid_id_provided_while_extracting:user')).toBe(
      'Invalid id provided (user)'
    )
  })

  it('returns unknown codes verbatim instead of hiding them', () => {
    expect(translateErrorCode('some_new_backend_code')).toBe('some_new_backend_code')
  })

  it('falls back to a generic message when there is no code', () => {
    expect(translateErrorCode(undefined)).toBe('Something went wrong')
  })
})

describe('croatian plural forms', () => {
  it('resolves singular, paucal and plural correctly', () => {
    setLocale('hr')
    const t = i18n.global.t
    expect(t('shares.groups.memberCount', 1)).toBe('1 član')
    expect(t('shares.groups.memberCount', 3)).toBe('3 člana')
    expect(t('shares.groups.memberCount', 5)).toBe('5 članova')
    expect(t('shares.groups.memberCount', 11)).toBe('11 članova')
    expect(t('shares.groups.memberCount', 21)).toBe('21 član')
  })
})

describe('locale aware formatting', () => {
  it('formats relative time in the active locale', () => {
    const now = 1700000000
    expect(formatRelative(now - 30, now)).toBe('just now')
    expect(formatRelative(now - 120, now)).toBe('2 minutes ago')

    setLocale('fr')
    expect(formatRelative(now - 120, now)).toBe('il y a 2 minutes')

    setLocale('de')
    expect(formatRelative(now - 3600 * 5, now)).toBe('vor 5 Stunden')
  })

  it('formats sizes with the locale decimal separator', () => {
    expect(formatSize(1536)).toBe('1.50 KB')

    setLocale('hr')
    expect(formatSize(1536)).toBe('1,50 KB')
  })
})
