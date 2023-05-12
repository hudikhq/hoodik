import * as cryptfns from '../cryptfns'
import type { store as cryptoStore } from '../crypto'
import * as login from './login'
import * as register from './register'
import * as lscache from 'lscache'
import type { NavigationFailure, Router } from 'vue-router'
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
  store: ReturnType<typeof login.store>,
  crypto: ReturnType<typeof cryptoStore>
): Promise<void | NavigationFailure> {
  if (!hasAuthentication(store)) {
    if (maybeCouldMakeRequests()) {
      try {
        logger.info('Trying to call self')
        await store.self(crypto)

        if (crypto.keypair.input) {
          return
        }
      } catch (e) {
        logger.info(`Moving to login after failed attempt to get self: ${e}`)
        router.push({ name: 'login' })
      }
    }

    if (cryptfns.hasEncryptedPrivateKey()) {
      logger.info('Moving to decrypt private key')
      return router.push({ name: 'decrypt' })
    }

    logger.info('Moving to login')
    return router.push({ name: 'login' })
  }
}
