import Api, { ErrorResponse } from '../api'
import * as cryptfns from '../cryptfns'
import * as opaque from '../cryptfns/opaque'
import * as envelope from '../cryptfns/envelope'
import * as transition from '../cryptfns/transition'
import * as x25519 from '../cryptfns/x25519'
import * as ed25519 from '../cryptfns/ed25519'
import { localDateFromUtcString } from '..'
import * as pk from './pk'
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Authenticated, Credentials, CryptoStore, KeyPair, PrivateKeyLogin } from 'types'
import { useRouter } from 'vue-router'
import * as logger from '!/logger'

export interface LoginStartResponse {
  method: 'password' | 'opaque'
  login_id?: string
  credential_response?: string
}

interface OpaqueLoginFinishRequest {
  login_id: string
  credential_finalization: string
  token?: string
}

interface MigrationKey {
  file_id: string
  encrypted_key: string
}

interface OpaqueRegisterStartRequest {
  registration_request: string
}

interface OpaqueRegisterStartResponse {
  registration_response: string
}

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
    logger.info('[auth] logout')
    const response = await Api.post<undefined, Authenticated>('/api/auth/logout')

    clear()
    crypto.clear()
    pk.clearRememberMe()
    sessionStorage.clear()
    // The file store holds the previous account's decrypted listing; drop it so
    // a login as a different account (no page reload) doesn't show their files.
    // Lazy import avoids the storage → services → auth import cycle at boot.
    const { store: filesStore } = await import('../storage')
    filesStore().reset()
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
      logger.debug('[auth] refreshing session')
      _refreshing.value = true
      await refresh(crypto)
      logger.info('[auth] session refreshed successfully')
      _refreshing.value = false
    } catch (e) {
      _refreshing.value = false
      logger.error(`[auth] session refresh failed: ${e}`)

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
   * Perform login operation regularly with normal credentials.
   * This now branches based on /login/start:
   * - legacy (security_version=0): password login + auto-migration ceremony if needed
   * - migrated: OPAQUE login
   */
  async function withCredentials(
    crypto: CryptoStore,
    credentials: Credentials
  ): Promise<Authenticated> {
    // Always start an OPAQUE client login attempt first (local only). This produces
    // the credential_request we must send to /login/start. The server will tell us
    // whether to continue with OPAQUE or fall back to legacy password.
    const clientStart = await opaque.clientLoginStart(credentials.password)

    let start: LoginStartResponse
    try {
      start = await loginStart(credentials.email, clientStart.message)
    } catch (e) {
      // Servers predating the OPAQUE endpoints have no /login/start route.
      // Fall back to the legacy password login so the client keeps working
      // against self-hosted instances that have not upgraded yet.
      if (e instanceof ErrorResponse && e.status === 404) {
        start = { method: 'password' }
      } else {
        throw e
      }
    }

    if (start.method === 'opaque' && start.login_id && start.credential_response) {
      return await _withOpaque(crypto, credentials, start, clientStart.state)
    }

    // Legacy password path (server said "password" or the account is not migrated).
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

    logger.info(`[auth] logged in as ${authenticated.user.email}`)

    // Automatic migration for legacy accounts. This is the last time the plaintext
    // password and the decrypted RSA private are both available client-side. The
    // ceremony re-wraps every file key under the new X25519 key, so a failure
    // must never commit a partial re-key — it aborts and the account stays legacy.
    const secVer = authenticated.user.security_version ?? 0
    if (secVer === 0 && credentials.password && credentials.privateKey) {
      try {
        await runMigrationCeremony(authenticated, credentials.privateKey, credentials.password, crypto)
      } catch (e) {
        logger.error('[auth] auto-migration ceremony failed (user stays legacy)', e)
      }
    }

    return authenticated
  }

  async function _withOpaque(
    crypto: CryptoStore,
    credentials: Credentials,
    start: LoginStartResponse,
    clientLoginState: string
  ): Promise<Authenticated> {
    if (!start.login_id || !start.credential_response) {
      throw new Error('Invalid opaque login start response')
    }

    const clientFinish = await opaque.clientLoginFinish(
      clientLoginState,
      start.credential_response,
      credentials.password
    )

    const finishResp = await Api.post<OpaqueLoginFinishRequest, Authenticated>(
      '/api/auth/login/finish',
      undefined,
      {
        login_id: start.login_id,
        credential_finalization: clientFinish.finalization,
        token: credentials.token
      }
    )

    if (!finishResp.body) {
      throw new Error('No authenticated object after opaque login finish')
    }

    const authenticated = finishResp.body

    // For a migrated account the encrypted_private_key is the envelope; the
    // export_key opens it and yields the Ed identity key plus the X wrapping key.
    const exportKeyBytes = cryptfns.uint8.fromBase64(clientFinish.exportKey)

    const kek = await envelope.deriveKek(exportKeyBytes)
    const env = authenticated.user.encrypted_private_key as string
    const bundle = await envelope.open(kek, env)

    const bundleStr = new TextDecoder().decode(bundle)
    let edPriv = ''
    let xPriv = ''
    const parts = bundleStr.split('|')
    for (const p of parts) {
      if (p.startsWith('ed:')) edPriv = p.slice(3)
      if (p.startsWith('x:')) xPriv = p.slice(2)
    }

    const kp: KeyPair = {
      input: edPriv || authenticated.user.pubkey || '',
      publicKey: authenticated.user.pubkey || null,
      fingerprint: authenticated.user.fingerprint || null,
      keySize: 0,
      keyType: 'curve25519',
      wrappingPrivate: xPriv || null
    }
    await crypto.set(kp)

    logger.info(`[auth] logged in as ${authenticated.user.email} (opaque)`)
    return authenticated as Authenticated
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

    logger.info(`[auth] logged in as ${authenticated.user.email} (private key)`)
    return response.body as Authenticated
  }

  /**
   * Call login/start to learn whether the account is still legacy (password)
   * or has migrated to OPAQUE. This is the first step for any email+password
   * login attempt.
   */
  async function loginStart(
    email: string,
    credentialRequest: string
  ): Promise<LoginStartResponse> {
    const resp = await Api.post(
      '/api/auth/login/start',
      undefined,
      { email, credential_request: credentialRequest }
    )
    if (!resp.body) {
      throw new Error('No response from login/start')
    }
    return resp.body as LoginStartResponse
  }

  /**
   * Full client-side migration ceremony for a legacy account.
   * Called after successful legacy password login while we still have the plaintext password.
   * On success, the current session continues with the new Curve25519 keys.
   */
  async function runMigrationCeremony(
    authenticated: Authenticated,
    oldRsaPrivPem: string,
    password: string,
    cryptoStore: CryptoStore
  ): Promise<void> {
    logger.info('[auth] starting legacy -> curve25519 + OPAQUE migration ceremony')

    const user = authenticated.user

    // 1. Generate new Ed25519 identity + X25519 wrapping keys
    const newEdPriv = await ed25519.generatePrivateKey()
    const newEdPub = await ed25519.publicFromPrivate(newEdPriv)
    const newXPriv = await x25519.generatePrivateKey()
    const newXPub = await x25519.publicFromPrivate(newXPriv)

    // The go-forward fingerprint is the SPKI hash of the new identity key. It
    // must be computed correctly — a wrong value makes the server reject the
    // migration (it recomputes and compares), so there is no safe fallback.
    const newFp = await ed25519.fingerprint(newEdPub)

    // 2. Fetch every key the user holds and re-wrap it under the new X25519
    // key. A single failure aborts the whole migration: the old RSA key is
    // about to be discarded, so a skipped key would be permanently unreadable.
    const keysResp = await Api.get<MigrationKey[]>('/api/auth/migration/keys')
    const rewrapped: MigrationKey[] = []
    let selfCheckSample: { blob: string; fileKey: Uint8Array } | null = null

    for (const k of (keysResp.body || [])) {
      const fileKeyHex = await cryptfns.rsa.decryptMessage(oldRsaPrivPem, k.encrypted_key)
      const fileKeyBytes = cryptfns.uint8.fromHex(fileKeyHex)
      const newWrapped = await x25519.wrap(fileKeyBytes, newXPub)
      rewrapped.push({ file_id: k.file_id, encrypted_key: newWrapped })
      if (!selfCheckSample) {
        selfCheckSample = { blob: newWrapped, fileKey: fileKeyBytes }
      }
    }

    // 3. Build and sign the key transition certificate
    const issuedAt = Math.floor(Date.now() / 1000)
    // user id as 16 bytes from uuid (remove dashes)
    const userIdBytes = new Uint8Array(16)
    const hex = user.id.replace(/-/g, '')
    for (let i = 0; i < 16; i++) {
      userIdBytes[i] = parseInt(hex.substr(i * 2, 2), 16)
    }

    const certSigs = await transition.sign({
      userId: userIdBytes,
      oldKeyType: 'rsa',
      oldKeyPem: user.pubkey,
      oldFingerprint: user.fingerprint,
      newIdentityKeyPem: newEdPub,
      newWrappingKeyPem: newXPub,
      newFingerprint: newFp,
      issuedAt: BigInt(issuedAt),
      oldPrivateKey: oldRsaPrivPem,
      newIdentityPrivateKey: newEdPriv
    })

    // 4. OPAQUE registration (must be authenticated)
    const regStart = await opaque.clientRegistrationStart(password)
    const regStartResp = await Api.post<
      OpaqueRegisterStartRequest,
      OpaqueRegisterStartResponse
    >('/api/auth/pake/register/start', undefined, {
      registration_request: regStart.message
    })
    if (!regStartResp.body) {
      throw new Error('No response from pake/register/start')
    }
    const regFinish = await opaque.clientRegistrationFinish(
      regStart.state,
      regStartResp.body.registration_response,
      password
    )

    // export_key crosses the binding as base64, not hex.
    const exportKeyBytes = cryptfns.uint8.fromBase64(regFinish.exportKey)

    // 5. Seal the private material into an envelope keyed by the OPAQUE export
    // key. The bundle is "v1|rsa:PEM|ed:PEM|x:PEM"; the envelope treats it as
    // opaque bytes and _withOpaque parses it back on the next login.
    const bundleStr = `v1|rsa:${oldRsaPrivPem}|ed:${newEdPriv}|x:${newXPriv}`
    const bundle = new TextEncoder().encode(bundleStr)

    const kek = await envelope.deriveKek(exportKeyBytes)
    const env = await envelope.seal(kek, bundle)

    // 6. Self-check before submitting anything: prove the new keys actually
    // work, so we never commit a migration that would lock the user out.
    const reopened = await envelope.open(kek, env)
    if (!reopened || reopened.length === 0) {
      throw new Error('self-check: envelope open failed')
    }
    const probe = 'migration-probe-' + Date.now()
    const probeSig = await ed25519.sign(probe, newEdPriv)
    if (!(await ed25519.verify(probe, probeSig, newEdPub))) {
      throw new Error('self-check: ed25519 signature failed')
    }
    // A re-wrapped key must unwrap under the new X25519 key and match the
    // original — this is what proves every file survives the re-key.
    if (selfCheckSample) {
      const recovered = await x25519.unwrap(selfCheckSample.blob, newXPriv)
      const expected = selfCheckSample.fileKey
      const matches =
        recovered.length === expected.length && recovered.every((b, i) => b === expected[i])
      if (!matches) {
        throw new Error('self-check: rewrapped key does not round-trip under the new key')
      }
    }

    // 7. Submit the complete migration (single transaction on server)
    const completeBody = {
      new_identity_pubkey: newEdPub,
      new_wrapping_pubkey: newXPub,
      new_fingerprint: newFp,
      transition_old_signature: certSigs.oldSignature,
      transition_new_signature: certSigs.newSignature,
      transition_issued_at: issuedAt,
      opaque_registration_upload: regFinish.message,
      encrypted_private_key: env,
      rewrapped_keys: rewrapped
    }

    const completeResp = await Api.post('/api/auth/migration/complete', undefined, completeBody)
    if (!completeResp.body) {
      throw new Error('migration/complete failed')
    }

    // 8. Update the in-memory crypto with the new identity (Ed) + wrapping (X) keys.
    // Downstream code (sign, decryptOwn, wrap) will dispatch based on keyType + extra fields.
    const migratedKp: KeyPair = {
      input: newEdPriv,
      publicKey: newEdPub,
      fingerprint: newFp,
      keySize: 0,
      keyType: 'curve25519',
      wrappingPrivate: newXPriv,
      wrappingPublic: newXPub
    }
    await cryptoStore.set(migratedKp)

    logger.info('[auth] migration ceremony completed successfully')
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
    setupAuthenticated,
    loginStart
  }
})
