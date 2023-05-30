import Api from '../api'
import * as cryptfns from '../cryptfns'
import { localDateFromUtcString } from '..'
import * as pk from './pk'
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Authenticated, Credentials, CryptoStore, KeyPair, PrivateKeyLogin } from 'types'
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
    pk.clearRememberMe()
  }

  /**
   * Setup the authenticated object after successful authentication event
   */
  async function setupAuthenticated(
    authenticated: Authenticated,
    privateKey: string,
    crypto: CryptoStore
  ) {
    set(authenticated)
    await crypto.set(privateKey)

    _refresher.value = setInterval(() => setupRefresh(crypto), 1000)
  }

  /**
   * Similar to setupAuthenticated, but it also stores the encrypted private key
   * if the user choose to be remembered. This way the session can stay alive
   * even if the user closes the browser.
   *
   * The private key is encrypted with a known device id, so it can be decrypted
   * when the session is refreshed.
   *
   * The downside of this approach is that if someone steals users JWT and refresh
   * token he will be able to decrypt the private key and use it to login.
   *
   * But that requires the attacker to gain access to HTTP only JWT and refresh cookies
   * + to gain access to localStorage where the encrypted private key is stored.
   *
   * This will only be delete out of the browser when user logs out.
   */
  async function setupAndRemember(
    authenticated: Authenticated,
    privateKey: string,
    crypto: CryptoStore
  ) {
    await pk.setRememberMe(privateKey, authenticated.session.device_id as string)

    return setupAuthenticated(authenticated, privateKey, crypto)
  }

  /**
   * Logout and the current user and delete everything stored about him
   * @throws
   */
  async function logout(crypto: CryptoStore, full?: boolean): Promise<Authenticated> {
    const response = await Api.post<undefined, Authenticated>('/api/auth/logout')

    clear()
    crypto.clear()
    pk.clearRememberMe()
    sessionStorage.clear()
    clearInterval(_refresher.value)

    if (full) {
      pk.clearPin()
    }

    return response.body as Authenticated
  }

  /**
   * Try to get the current user
   * @throws
   * @deprecated Use refresh() instead where the same session will be simply refreshed
   * if the jwt is expired, this one only tries to get the authenticated using the jwt
   * which will probably be expired once you try to get it with this function.
   */
  async function self(store: CryptoStore): Promise<Authenticated> {
    const response = await Api.post<undefined, Authenticated>('/api/auth/self')
    const authenticated = response.body as Authenticated

    const privateKey = await pk.getRememberMe(
      authenticated.session.device_id as string,
      authenticated.user.fingerprint
    )

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
  async function setupRefresh(crypto: CryptoStore): Promise<void> {
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
      await refresh(crypto)
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
  async function refresh(crypto: CryptoStore): Promise<Authenticated> {
    const response = await Api.post<undefined, Authenticated>('/api/auth/refresh')

    if (!response.body) {
      throw new Error("No authenticated object found after refresh, can't refresh session")
    }

    let privateKey = await pk.getRememberMe(
      response.body.session?.device_id as string,
      response.body.user.fingerprint
    )

    if (!privateKey) {
      privateKey = crypto.keypair.input
    }

    if (!privateKey) {
      throw new Error(
        'No private key found, please provide your private key when authenticating again'
      )
    }

    await setupAuthenticated(response.body as Authenticated, privateKey, crypto)

    return response.body as Authenticated
  }

  /**
   * Perform login operation regularly with normal credentials
   * @throws
   */
  async function withCredentials(
    crypto: CryptoStore,
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

    if (credentials.remember) {
      await setupAndRemember(authenticated, keypair.input as string, crypto)
    } else {
      await setupAuthenticated(authenticated, keypair.input as string, crypto)
    }

    return authenticated
  }

  /**
   * Takes the given private key and passphrase, tries to decrypt it and then perform authentication
   * @throws
   */
  async function withPrivateKey(
    store: CryptoStore,
    input: PrivateKeyLogin
  ): Promise<Authenticated> {
    const { privateKey } = input

    const pk = privateKey

    return _withPrivateKey(store, await cryptfns.rsa.inputToKeyPair(pk || ''), !!input.remember)
  }

  /**
   * Attempt to decrypt the private key and get the current user from backend
   * @throws
   */
  async function withPin(store: CryptoStore, pin: string): Promise<Authenticated> {
    const privateKey = await pk.getPinAndDecrypt(pin)

    return _withPrivateKey(store, await cryptfns.rsa.inputToKeyPair(privateKey), false)
  }

  /**
   * Perform authentication with KeyPair object, performs fingerprint calculation and signature creation
   * @throws
   */
  async function _withPrivateKey(
    crypto: CryptoStore,
    keypair: KeyPair,
    remember: boolean
  ): Promise<Authenticated> {
    const fingerprint = await cryptfns.rsa.getFingerprint(keypair.input as string)
    const nonce = cryptfns.createFingerprintNonce(fingerprint)
    const signature = await cryptfns.rsa.sign(keypair, nonce)

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

    const authenticated = response.body

    if (remember) {
      await setupAndRemember(authenticated, keypair.input as string, crypto)
    } else {
      await setupAuthenticated(authenticated, keypair.input as string, crypto)
    }

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
