/**
 * Setting up a mobile app against this instance.
 *
 * Two ways in, depending on what the user is holding. On a desktop they scan
 * a QR code with their phone. On the phone itself a QR is useless — nobody can
 * scan the screen they're reading — so they get a button that opens the app,
 * or the store when it isn't installed.
 *
 * Nothing here is a secret: it saves typing a server address, the password is
 * still entered on the phone.
 */

const APP_STORE = 'https://apps.apple.com/app/hoodik/id6761471179'
const PLAY_STORE = 'https://play.google.com/store/apps/details?id=com.hudikhq.hoodik'

/** How long to wait for the app to take over before deciding it isn't installed. */
const APP_OPEN_GRACE_MS = 2000

function params(origin: string, email: string): string {
  return new URLSearchParams({ s: origin, e: email }).toString()
}

/**
 * The URL a QR code carries. It points at hoodik.io rather than this instance
 * because it has to work for a phone with no app installed, which needs a
 * store link this server can't give it.
 *
 * Both values ride in the fragment, and fragments are never sent in an HTTP
 * request — so hoodik.io can route someone onto their own instance without
 * learning which instance it is or who they are.
 */
export function connectUrl(origin: string, email: string): string {
  return `https://hoodik.io/connect#${params(origin, email)}`
}

/** Straight to the app, for a device that already has it. */
export function appConnectUrl(origin: string, email: string): string {
  return `hoodik://connect#${params(origin, email)}`
}

export type MobilePlatform = 'ios' | 'android' | null

export function mobilePlatform(userAgent: string = navigator.userAgent): MobilePlatform {
  if (/iPhone|iPad|iPod/.test(userAgent)) return 'ios'
  if (/Android/.test(userAgent)) return 'android'

  return null
}

export function storeUrl(platform: MobilePlatform): string {
  return platform === 'ios' ? APP_STORE : PLAY_STORE
}

/**
 * Open the app on this device, landing in the store when it isn't installed.
 *
 * A custom scheme that nothing handles fails silently, so the store is armed
 * on a timer and cancelled the moment the app takes the foreground — which is
 * what hiding the page means here. The listeners are what keep a slow tap on
 * iOS's "Open in Hoodik?" prompt from being mistaken for a missing app.
 */
export function openInApp(origin: string, email: string, platform = mobilePlatform()): void {
  const fallback = window.setTimeout(() => {
    window.location.href = storeUrl(platform)
  }, APP_OPEN_GRACE_MS)

  const cancel = () => {
    if (document.hidden) window.clearTimeout(fallback)
  }

  document.addEventListener('visibilitychange', cancel, { once: true })
  window.addEventListener('pagehide', () => window.clearTimeout(fallback), { once: true })

  window.location.href = appConnectUrl(origin, email)
}

const PROMPT_KEY_PREFIX = 'hoodik:connectPrompt:'

/**
 * Whether the user has already dealt with the one-time prompt offering to set
 * up the app. Per browser, which is the right grain — a new browser is a new
 * device, and the phone in your pocket may well not be set up there yet.
 */
export function isPromptDismissed(userId: string): boolean {
  try {
    return localStorage.getItem(`${PROMPT_KEY_PREFIX}${userId}`) === '1'
  } catch {
    // A private-mode browser with no storage just sees the prompt each time.
    return false
  }
}

export function dismissPrompt(userId: string): void {
  try {
    localStorage.setItem(`${PROMPT_KEY_PREFIX}${userId}`, '1')
  } catch {
    // Nothing to persist to; the prompt stays dismissible in-session.
  }
}
