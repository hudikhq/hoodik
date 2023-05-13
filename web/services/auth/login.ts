import Api from '../api'
import * as cryptfns from '../cryptfns'
import { localDateFromUtcString } from '..'
import { setPrivateKey, getPrivateKey, removePrivateKey } from '.'
import type { store as cryptoStore } from '../crypto'
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Authenticated, Credentials, KeyPair, PrivateKeyLogin } from 'types'
import { useRouter } from 'vue-router'
import * as logger from '!/logger'

interface PrivateKeyRequest {
  fingerprint: string
  signature: string
  remember: boolean
}

export const store = defineStore('login', () => {
  const _authenticated = ref<Authenticated | null>(null)
  const _refresher = ref()
  const _refreshing = ref(false)

  const authenticated = computed<Authenticated | null>(() => _authenticated.value)

  /**
   * Set Authenticated object
   */
  function set(auth: Authenticated) {
    if (auth.session.device_id) {
      delete auth.session.device_id
    }

    _authenticated.value = auth
  }

  /**
   * Clear Authenticated object
   */
  function clear() {
    _authenticated.value = null
    removePrivateKey()
  }

  /**
   * Setup the authenticated object after successful authentication event
   */
  async function setupAuthenticated(authenticated: Authenticated, privateKey: string) {
    const expires = localDateFromUtcString(authenticated.session.expires_at)

    await setPrivateKey(privateKey, authenticated.session.device_id as string, expires)
    set(authenticated)

    _refresher.value = setInterval(() => setupRefresh(), 1000)
  }

  /**
   * Logout and the current user and delete everything stored about him
   * @throws
   */
  async function logout(
    crypto: ReturnType<typeof cryptoStore>,
    full?: boolean
  ): Promise<Authenticated> {
    const response = await Api.post<undefined, Authenticated>('/api/auth/logout')

    clear()
    crypto.clear()
    sessionStorage.clear()
    clearInterval(_refresher.value)

    if (full) {
      crypto.clear()
    }

    return response.body as Authenticated
  }

  /**
   * Try to get the current user
   * @throws
   */
  async function self(store: ReturnType<typeof cryptoStore>): Promise<Authenticated> {
    const response = await Api.post<undefined, Authenticated>('/api/auth/self')
    const authenticated = response.body as Authenticated

    const privateKey = await getPrivateKey(authenticated.session.device_id as string)

    if (privateKey) {
      const fingerprint = await cryptfns.rsa.getFingerprint(privateKey)
      if (fingerprint === authenticated.user.fingerprint) {
        const keypair = await cryptfns.rsa.inputToKeyPair(privateKey)

        return _withPrivateKey(store, keypair, false)
      }
    }

    throw new Error(`No private key found for user ${authenticated.user.email}`)
  }

  /**
   * Attempt to refresh the session
   */
  async function setupRefresh(): Promise<void> {
    const expires = authenticated.value?.session.expires_at

    if (!expires) {
      return
    }

    const expiresAt = localDateFromUtcString(expires)
    const now = new Date().getTime()

    const untilExpire = (expiresAt.getTime() - now) / 1000

    if (untilExpire > 60) {
      return
    }

    if (_refreshing.value) {
      return
    }

    try {
      logger.debug('Attempting to refresh the session')
      _refreshing.value = true
      await refresh()
      _refreshing.value = false
    } catch (e) {
      _refreshing.value = false
      logger.error(`Error when attempting to refresh session: ${e}`)

      clear()

      useRouter().push({ name: 'login' })
    }
  }

  /**
   * Try to get the current user
   * @throws
   */
  async function refresh(): Promise<Authenticated> {
    const response = await Api.post<undefined, Authenticated>('/api/auth/refresh')

    if (!response.body) {
      throw new Error("No authenticated object found after refresh, can't refresh session")
    }

    const privateKey = await getPrivateKey(response.body?.session?.device_id as string)

    if (!privateKey) {
      throw new Error(
        'No private key found, please provide your private key when authenticating again'
      )
    }

    await setupAuthenticated(response.body as Authenticated, privateKey)

    return response.body as Authenticated
  }

  /**
   * Perform login operation regularly with normal credentials
   * @throws
   */
  async function withCredentials(
    store: ReturnType<typeof cryptoStore>,
    credentials: Credentials
  ): Promise<Authenticated> {
    const response = await Api.post<Credentials, Authenticated>(
      '/api/auth/login',
      undefined,
      credentials
    )

    if (!response.body) {
      throw new Error('No authenticated object found after login')
    }

    const authenticated = response.body

    if (authenticated.user.encrypted_private_key) {
      credentials.privateKey = await cryptfns.rsa.decryptPrivateKey(
        authenticated.user.encrypted_private_key,
        credentials.password
      )
    }

    if (!credentials.privateKey) {
      throw new Error('No private key found, please provide your private key when authenticating')
    }

    const fingerprint = await cryptfns.rsa.getFingerprint(credentials.privateKey)
    if (fingerprint !== authenticated.user.fingerprint) {
      throw new Error('Private key does not match user')
    }

    const keypair = await cryptfns.rsa.inputToKeyPair(credentials.privateKey)

    await setupAuthenticated(authenticated, keypair.input as string)

    await store.set(keypair)

    return authenticated
  }

  /**
   * Takes the given private key and passphrase, tries to decrypt it and then perform authentication
   * @throws
   */
  async function withPrivateKey(
    store: ReturnType<typeof cryptoStore>,
    input: PrivateKeyLogin
  ): Promise<Authenticated> {
    const { privateKey } = input

    const pk = privateKey

    return _withPrivateKey(store, await cryptfns.rsa.inputToKeyPair(pk || ''), false)
  }

  /**
   * Attempt to decrypt the private key and get the current user from backend
   * @throws
   */
  async function withPin(
    store: ReturnType<typeof cryptoStore>,
    pin: string
  ): Promise<Authenticated> {
    const pk = await cryptfns.getAndDecryptPrivateKey(pin)

    return _withPrivateKey(store, await cryptfns.rsa.inputToKeyPair(pk), false)
  }

  /**
   * Perform authentication with KeyPair object, performs fingerprint calculation and signature creation
   * @throws
   */
  async function _withPrivateKey(
    store: ReturnType<typeof cryptoStore>,
    kp: KeyPair,
    remember: boolean
  ): Promise<Authenticated> {
    const fingerprint = await cryptfns.rsa.getFingerprint(kp.input as string)
    const nonce = cryptfns.createFingerprintNonce(fingerprint)
    const signature = await cryptfns.rsa.sign(kp, nonce)

    const response = await Api.post<PrivateKeyRequest, Authenticated>(
      '/api/auth/signature',
      {},
      {
        fingerprint,
        signature,
        remember
      }
    )

    if (!response.body) {
      throw new Error('No authenticated object found after private key or pin login')
    }

    await setupAuthenticated(response.body as Authenticated, kp.input as string)

    await store.set(kp)

    return response.body as Authenticated
  }

  return {
    authenticated,
    set,
    clear,
    self,
    refresh,
    logout,
    withCredentials,
    withPrivateKey,
    withPin,
    setupAuthenticated
  }
})
