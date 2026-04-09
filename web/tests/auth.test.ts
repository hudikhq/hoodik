import { describe, it, expect, beforeEach } from 'vitest'
import { popIntendedRoute } from '../services/auth'

const REDIRECT_KEY = 'hoodik:auth:redirect'

describe('popIntendedRoute', () => {
  beforeEach(() => {
    sessionStorage.clear()
  })

  it('UNIT: Returns a valid local path', () => {
    sessionStorage.setItem(REDIRECT_KEY, '/files/abc123')
    expect(popIntendedRoute()).toBe('/files/abc123')
  })

  it('UNIT: Returns null when sessionStorage is empty', () => {
    expect(popIntendedRoute()).toBeNull()
  })

  it('UNIT: Clears sessionStorage after reading', () => {
    sessionStorage.setItem(REDIRECT_KEY, '/files/abc123')
    popIntendedRoute()
    expect(sessionStorage.getItem(REDIRECT_KEY)).toBeNull()
  })

  it('UNIT: Rejects protocol-relative URL (//evil.com)', () => {
    sessionStorage.setItem(REDIRECT_KEY, '//evil.com')
    expect(popIntendedRoute()).toBeNull()
  })

  it('UNIT: Rejects absolute URL (https://evil.com)', () => {
    sessionStorage.setItem(REDIRECT_KEY, 'https://evil.com')
    expect(popIntendedRoute()).toBeNull()
  })

  it('UNIT: Rejects empty string', () => {
    sessionStorage.setItem(REDIRECT_KEY, '')
    expect(popIntendedRoute()).toBeNull()
  })

  it('UNIT: Returns a path with query params and hash', () => {
    sessionStorage.setItem(REDIRECT_KEY, '/files?sort=name#section')
    expect(popIntendedRoute()).toBe('/files?sort=name#section')
  })
})
