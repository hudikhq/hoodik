import * as login from './login'
import * as register from './register'
import * as pk from './pk'
import type { NavigationFailure, RouteLocationNormalizedLoaded, Router } from 'vue-router'
import * as logger from '!/logger'
import type { CryptoStore, LoginStore } from 'types'

import { store as cryptoStore } from '!/crypto'

export { login, register, pk }

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
