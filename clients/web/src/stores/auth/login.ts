import Api from '../api'
import * as cryptfns from '../cryptfns'
import { localDateFromUtcString } from '..'
import { setJwt, setCsrf, removeJwt, removeCsrf } from '.'
import type { store as cryptoStore } from '../crypto'
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface Authenticated {
  user: User
  session: Session
}

export interface AuthenticatedJwt {
  authenticated: Authenticated
  jwt: string
}

export interface User {
  id: number
  email: string
  private?: string
  pubkey: string
  fingerprint: string
  encrypted_private_key?: string
  created_at: string
  updated_at: string
}

export interface Session {
  id: number
  user_id: number
  token: string
  csrf: string
  created_at: string
  updated_at: string
  expires_at: string
}

export interface Credentials {
  email: string
  password: string
  token?: string
  remember?: boolean
  privateKey?: string
}

export interface PrivateKeyLogin {
  privateKey: string
  remember?: boolean
}

interface PrivateKeyRequest {
  fingerprint: string
  signature: string
  remember: boolean
}

export const store = defineStore('login', () => {
  const _authenticated = ref<Authenticated | null>(null)
  const _refresher = ref()

  const authenticated = computed<Authenticated | null>(() => _authenticated.value)

  /**
   * Set Authenticated object
   */
  function set(auth: Authenticated) {
    _authenticated.value = auth
  }

  /**
   * Clear Authenticated object
   */
  function clear() {
    _authenticated.value = null
    removeJwt()
    removeCsrf()
  }

  /**
   * Setup the authenticated object after successful authentication event
   */
  function setupAuthenticated(body: AuthenticatedJwt, privateKey?: string | null) {
    const { authenticated, jwt } = body

    if (privateKey) {
      sessionStorage.setItem('privateKey', privateKey)
    }

    const expires = localDateFromUtcString(authenticated.session.expires_at)

    setCsrf(authenticated.session.csrf, expires)
    setJwt(jwt, expires)
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

    const privateKey = sessionStorage.getItem('privateKey')

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

    const leftSeconds = (expiresAt.getTime() - new Date().getTime()) / 1000

    if (leftSeconds > 30) {
      return
    }

    try {
      console.info('Attempting to refresh the session')
      await refresh()
    } catch (e) {
      console.error(`Error when attempting to refresh session: ${e}`)

      clear()
    }
  }

  /**
   * Try to get the current user
   * @throws
   */
  async function refresh(): Promise<Authenticated> {
    const response = await Api.post<undefined, AuthenticatedJwt>('/api/auth/refresh')

    if (!response.body?.authenticated || !response.body?.jwt) {
      throw new Error("No authenticated object found after refresh, can't refresh session")
    }

    const privateKey = sessionStorage.getItem('privateKey')
    setupAuthenticated(response.body as AuthenticatedJwt, privateKey)

    return response.body?.authenticated as Authenticated
  }

  /**
   * Perform login operation regularly with normal credentials
   * @throws
   */
  async function withCredentials(
    store: ReturnType<typeof cryptoStore>,
    credentials: Credentials
  ): Promise<Authenticated> {
    const response = await Api.post<Credentials, AuthenticatedJwt>(
      '/api/auth/login',
      undefined,
      credentials
    )

    if (!response.body?.authenticated || !response.body?.jwt) {
      throw new Error('No authenticated object found after login')
    }

    const { authenticated } = response.body

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

    setupAuthenticated(response.body, keypair.input)

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
    const pk = cryptfns.getAndDecryptPrivateKey(pin)

    return _withPrivateKey(store, await cryptfns.rsa.inputToKeyPair(pk), false)
  }

  /**
   * Perform authentication with KeyPair object, performs fingerprint calculation and signature creation
   * @throws
   */
  async function _withPrivateKey(
    store: ReturnType<typeof cryptoStore>,
    kp: cryptfns.rsa.KeyPair,
    remember: boolean
  ): Promise<Authenticated> {
    const fingerprint = await cryptfns.rsa.getFingerprint(kp.input as string)
    const nonce = cryptfns.createFingerprintNonce(fingerprint)
    const signature = await cryptfns.rsa.sign(kp, nonce)

    const response = await Api.post<PrivateKeyRequest, AuthenticatedJwt>(
      '/api/auth/signature',
      {},
      {
        fingerprint,
        signature,
        remember
      }
    )

    if (!response.body?.authenticated || !response.body?.jwt) {
      throw new Error('No authenticated object found after private key or pin login')
    }

    setupAuthenticated(response.body as AuthenticatedJwt, kp.input)

    await store.set(kp)

    return response.body.authenticated
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
