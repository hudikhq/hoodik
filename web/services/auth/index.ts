import * as cryptfns from '../cryptfns'
import type { store as cryptoStore } from '../crypto'
import * as login from './login'
import * as register from './register'
import * as lscache from 'lscache'
import type { NavigationFailure, RouteLocationNormalizedLoaded, Router } from 'vue-router'
import * as logger from '!/logger'

export { login, register }

const PRIVATE_KEY_STORE_NAME = 'ENCRYPTED-PRIVATE-KEY'

/**
 * Shortcut to figure out if we can make requests
 */
export function maybeCouldMakeRequests(): boolean {
  return true
}

/**
 * Get the private key from storage and decrypt it
 */
export async function getPrivateKey(deviceId: string): Promise<string | null> {
  const privateKey = lscache.get(PRIVATE_KEY_STORE_NAME) || null

  if (!privateKey) {
    return null
  }

  return cryptfns.aes.decryptString(privateKey, deviceId)
}

/**
 * Set the private key in storage and encrypt it
 */
export async function setPrivateKey(privateKey: string, deviceId: string, expires: Date) {
  const ex = expires.getTime() - new Date().getTime()
  lscache.set(PRIVATE_KEY_STORE_NAME, await cryptfns.aes.encryptString(privateKey, deviceId), ex)
}

/**
 * Remove the private key
 */
export function removePrivateKey(): void {
  return lscache.remove(PRIVATE_KEY_STORE_NAME)
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
  route: RouteLocationNormalizedLoaded,
  store: ReturnType<typeof login.store>,
  crypto: ReturnType<typeof cryptoStore>
): Promise<void | NavigationFailure> {
  if (!hasAuthentication(store)) {
    logger.debug('No authenticated in the store')

    if (maybeCouldMakeRequests()) {
      logger.debug('Trying to call refresh')

      try {
        await store.refresh()

        if (crypto.keypair.input) {
          return
        }
      } catch (e) {
        return bounce(router, route, store, crypto)
      }
    }

    if (cryptfns.hasEncryptedPrivateKey()) {
      return decrypt(router, route)
    }

    return bounce(router, route, store, crypto)
  }
}

/**
 * One final try to attempt to purge the session and move to login page
 */
async function bounce(
  router: Router,
  route: RouteLocationNormalizedLoaded,
  store: ReturnType<typeof login.store>,
  crypto: ReturnType<typeof cryptoStore>,
  e?: Error
) {
  try {
    await crypto.clear()
    await store.logout(crypto, true)
  } catch (e) {
    // do nothing
  }

  if (e) {
    logger.debug(`Moving to login after error: ${e}`)
  } else {
    logger.debug('Moving to login')
  }

  if (route.name !== 'login') {
    router.push({ name: 'login', replace: true })
  } else {
    logger.debug('Already on login page, doing nothing')
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
  router.push({ name: 'decrypt', replace: true })
}
