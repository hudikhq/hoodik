import * as login from './login'
import * as register from './register'
import * as pk from './pk'
import type { NavigationFailure, RouteLocationNormalizedLoaded, Router } from 'vue-router'
import * as logger from '!/logger'
import type { CryptoStore, LoginStore } from 'types'

import { store as cryptoStore } from '!/crypto'

export { login, register, pk }

const REDIRECT_KEY = 'hoodik:auth:redirect'

/**
 * Save the current route path so we can redirect back after auth.
 * Only saves non-auth routes.
 */
function saveIntendedRoute(route: RouteLocationNormalizedLoaded) {
  const name = String(route.name || '')
  const authRoutes = ['login', 'login-private-key', 'decrypt', 'lock', 'setup-lock-screen', 'register', 'register-key', 'register-two-factor', 'register-resend-activation', 'forgot-password', 'activate-email']
  if (authRoutes.includes(name)) return

  sessionStorage.setItem(REDIRECT_KEY, route.fullPath)
}

/**
 * Pop the saved redirect path, returning it and clearing storage.
 */
export function popIntendedRoute(): string | null {
  const path = sessionStorage.getItem(REDIRECT_KEY)
  sessionStorage.removeItem(REDIRECT_KEY)

  // Only allow local paths to prevent open-redirect attacks
  if (path && path.startsWith('/') && !path.startsWith('//')) return path
  return null
}

/**
 * Shortcut to figure out if we can make requests
 */
export function maybeCouldMakeRequests(): boolean {
  return pk.hasRememberMe()
}

/**
 * Do we have authentication currently loaded?
 */
export function hasAuthentication(store: LoginStore) {
  return !!store.authenticated
}

/**
 * Ensure we have authentication and move user to appropriate pages if not
 */
export async function ensureAuthenticated(
  router: Router,
  route: RouteLocationNormalizedLoaded
): Promise<void | NavigationFailure> {
  const store = login.store()
  const crypto = cryptoStore()

  if (!store.authenticated) {
    logger.debug('No authenticated in the store')

    if (maybeCouldMakeRequests()) {
      logger.debug('Trying to call refresh')

      try {
        await store.refresh(crypto)

        if (crypto.keypair.input) {
          return
        }
      } catch (e) {
        return bounce(router, route, crypto)
      }
    }

    if (pk.hasPin()) {
      return decrypt(router, route)
    }

    return bounce(router, route, crypto)
  }
}

/**
 * Ensure current user is a staff user
 */
export async function ensureAdmin(
  router: Router,
  route: RouteLocationNormalizedLoaded
): Promise<void | NavigationFailure> {
  const store = login.store()

  await ensureAuthenticated(router, route)

  if (!store.authenticated?.user.role) {
    return router.push({ name: 'files', replace: true })
  }
}

/**
 * One final try to attempt to purge the session and move to login page
 */
async function bounce(
  router: Router,
  route: RouteLocationNormalizedLoaded,
  crypto: CryptoStore,
  e?: Error
) {
  try {
    await crypto.clear()
    await pk.clearRememberMe()
  } catch (e) {
    // do nothing
  }

  if (e) {
    logger.warn(`[auth] redirecting to login after error: ${e}`)
  } else {
    logger.warn('[auth] redirecting to login (no active session)')
  }

  if (route.name !== 'login') {
    saveIntendedRoute(route)
    router.push({ name: 'login', replace: true })
  } else {
    logger.debug('[auth] already on login page, doing nothing')
  }
}

/**
 * Push the user to decrypt page
 */
async function decrypt(router: Router, route: RouteLocationNormalizedLoaded) {
  if (route.name === 'decrypt') {
    logger.debug('Already on decrypt page, doing nothing')
    return
  }

  logger.debug('Moving to decrypt private key')
  saveIntendedRoute(route)
  router.push({ name: 'decrypt', replace: true })
}
