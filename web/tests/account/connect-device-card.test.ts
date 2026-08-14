import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'

import {
  appConnectUrl,
  connectUrl,
  dismissPrompt,
  isPromptDismissed,
  mobilePlatform,
  openInApp,
  storeUrl
} from '../../services/connect'
import ConnectDevice from '../../src/views/account/index/ConnectDevice.vue'
import type { User } from 'types'

const user = { id: 'u1', email: 'someone@example.com' } as User

const IPHONE = 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15'
const PIXEL = 'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 Chrome/148.0.0.0 Mobile'
const MAC = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 Chrome/148.0.0.0'

describe('connectUrl', () => {
  it('points at the instance the user is signed in to', () => {
    const url = new URL(connectUrl('https://drive.example.com', 'someone@example.com'))

    expect(url.origin + url.pathname).toBe('https://hoodik.io/connect')

    const params = new URLSearchParams(url.hash.slice(1))
    expect(params.get('s')).toBe('https://drive.example.com')
    expect(params.get('e')).toBe('someone@example.com')
  })

  /**
   * The whole privacy claim rests on this: hoodik.io catches the scan for
   * people without the app, and a fragment is never sent in an HTTP request.
   * Move either value into the query string and the landing page starts
   * learning which instance a user is on and who they are.
   */
  it('keeps the server and email out of the query string', () => {
    const url = new URL(connectUrl('https://drive.example.com', 'someone@example.com'))

    expect(url.search).toBe('')
    expect(url.hash).toContain('s=')
    expect(url.hash).toContain('e=')
  })

  it('hands the app the same payload without the hoodik.io detour', () => {
    const app = appConnectUrl('https://drive.example.com', 'someone@example.com')

    expect(app.startsWith('hoodik://connect#')).toBe(true)
    expect(new URLSearchParams(app.split('#')[1]).get('s')).toBe('https://drive.example.com')
  })
})

describe('mobilePlatform', () => {
  it('tells the two phones apart, and neither from a desktop', () => {
    expect(mobilePlatform(IPHONE)).toBe('ios')
    expect(mobilePlatform(PIXEL)).toBe('android')
    expect(mobilePlatform(MAC)).toBe(null)
  })

  it('sends each phone to its own store', () => {
    expect(storeUrl('ios')).toContain('apps.apple.com')
    expect(storeUrl('android')).toContain('play.google.com')
  })
})

describe('openInApp', () => {
  beforeEach(() => vi.useFakeTimers())

  // The location spy has to come off, or every later suite reads an origin of
  // undefined out of it.
  afterEach(() => {
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  it('falls back to the store when nothing handles the scheme', () => {
    const assigned: string[] = []
    vi.spyOn(window, 'location', 'get').mockReturnValue({
      set href(value: string) {
        assigned.push(value)
      },
      get href() {
        return assigned[assigned.length - 1] ?? ''
      }
    } as Location)

    openInApp('https://drive.example.com', 'someone@example.com', 'android')
    expect(assigned[0].startsWith('hoodik://connect#')).toBe(true)

    vi.advanceTimersByTime(2000)
    expect(assigned[1]).toContain('play.google.com')
  })

  /**
   * The app taking the foreground hides the page. Without this the store would
   * load underneath someone who is slowly tapping iOS's "Open in Hoodik?".
   */
  it('stays put once the app has taken over', () => {
    const assigned: string[] = []
    vi.spyOn(window, 'location', 'get').mockReturnValue({
      set href(value: string) {
        assigned.push(value)
      },
      get href() {
        return assigned[assigned.length - 1] ?? ''
      }
    } as Location)
    vi.spyOn(document, 'hidden', 'get').mockReturnValue(true)

    openInApp('https://drive.example.com', 'someone@example.com', 'ios')
    document.dispatchEvent(new Event('visibilitychange'))
    vi.advanceTimersByTime(5000)

    expect(assigned).toHaveLength(1)
    expect(assigned[0].startsWith('hoodik://connect#')).toBe(true)
  })
})

describe('connect prompt dismissal', () => {
  beforeEach(() => localStorage.clear())

  it('is undismissed until the user says so, per account', () => {
    expect(isPromptDismissed('u1')).toBe(false)

    dismissPrompt('u1')

    expect(isPromptDismissed('u1')).toBe(true)
    expect(isPromptDismissed('u2')).toBe(false)
  })
})

describe('ConnectDevice card', () => {
  it('encodes the current origin and the signed-in address', () => {
    const value = mount(ConnectDevice, { props: { user } })
      .findComponent({ name: 'Qrcode' })
      .props('value') as string

    const params = new URLSearchParams(new URL(value).hash.slice(1))
    expect(params.get('s')).toBe(window.location.origin)
    expect(params.get('e')).toBe('someone@example.com')
  })

  it('offers the tap-through only where a QR code is unusable', () => {
    // jsdom's default agent is a desktop one, so the phone-only control is out.
    const wrapper = mount(ConnectDevice, { props: { user } })

    expect(wrapper.find('[data-testid="account-connect-link"]').exists()).toBe(false)
  })
})
