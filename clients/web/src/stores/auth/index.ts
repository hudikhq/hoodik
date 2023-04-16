import * as cryptfns from '../cryptfns'
import type { store as cryptoStore } from '../crypto'
import * as login from './login'
import * as register from './register'
import Cookies from 'js-cookie'
import * as lscache from 'lscache'
import type { NavigationFailure, Router } from 'vue-router'

export { login, register }

const CSRF_COOKIE_NAME = 'X-CSRF-TOKEN'
const JWT_TOKEN_COOKIE_NAME = 'JWT-TOKEN'

/**
 * Shortcut to figure out if we can make requests
 */
export function maybeCouldMakeRequests(): boolean {
  return !!getCsrf() && !!getJwt()
}

/**
 * Load the CSRF token from the cookie
 */
export function getCsrf(): string | null {
  return Cookies.get(CSRF_COOKIE_NAME) || null
}

/**
 * Set the CSRF token into cookie
 */
export function setCsrf(csrf: string, expires: Date) {
  Cookies.set(CSRF_COOKIE_NAME, csrf, {
    path: '/',
    sameSite: 'lax',
    domain: import.meta.env.APP_COOKIE_DOMAIN,
    expires
  })
}

/**
 * Remove csrf cookie
 */
export function removeCsrf() {
  Cookies.remove(CSRF_COOKIE_NAME)
}

/**
 * Load the JWT token from the cookie
 */
export function getJwt(): string | null {
  return lscache.get(JWT_TOKEN_COOKIE_NAME) || null
}

/**
 * Set the JWT token into cookie
 */
export function setJwt(jwt: string, expires: Date) {
  const ex = expires.getTime() - new Date().getTime()
  lscache.set(JWT_TOKEN_COOKIE_NAME, jwt, ex)
}

/**
 * Remove the JWT token
 */
export function removeJwt(): void {
  return lscache.remove(JWT_TOKEN_COOKIE_NAME)
}

/**
 * Do we have authentication currently loaded?
 */
export function hasAuthentication(store: ReturnType<typeof login.store>) {
  return !!store.authenticated
}

/**
 * Ensure we have authentication and move user to appropriate pages if not
 */
export async function ensureAuthenticated(
  router: Router,
  store: ReturnType<typeof login.store>,
  crypto: ReturnType<typeof cryptoStore>
): Promise<void | NavigationFailure> {
  if (!hasAuthentication(store)) {
    if (maybeCouldMakeRequests()) {
      try {
        await store.self()

        if (crypto.keypair.input) {
          return
        }
      } catch (e) {
        console.info(`Moving to login after failed attempt to get self: ${e}`)
        router.push('/auth/login')
      }
    }

    if (cryptfns.hasEncryptedPrivateKey()) {
      console.info('Moving to decrypt private key')
      return router.push('/auth/decrypt')
    }

    console.info('Moving to login')
    return router.push('/auth/login')
  }
}
