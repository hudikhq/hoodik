import Api, { ErrorResponse } from '../api'
import * as cryptfns from '../cryptfns'
import * as opaque from '../cryptfns/opaque'
import * as envelope from '../cryptfns/envelope'
import * as transition from '../cryptfns/transition'
import * as wrapping from '../cryptfns/wrapping'
import * as ed25519 from '../cryptfns/ed25519'
import { localDateFromUtcString } from '..'
import * as pk from './pk'
import * as pkBundle from './bundle'
import * as migrationNotice from './migration-notice'
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { Authenticated, Credentials, CryptoStore, KeyPair, PrivateKeyLogin, User } from 'types'
import { useRouter } from 'vue-router'
import { notify } from '@kyvg/vue3-notification'
import * as logger from '!/logger'

export interface LoginStartResponse {
  method: 'password' | 'opaque'
  login_id?: string
  credential_response?: string
  ksf?: opaque.KsfParams
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

interface MigrationLinkKey {
  link_id: string
  encrypted_link_key: string
  file_id: string
}

interface RewrappedLinkKey {
  link_id: string
  encrypted_link_key: string
  signature: string
}

interface MigrationKeysResponse {
  keys: MigrationKey[]
  link_keys: MigrationLinkKey[]
  next_offset: number | null
}

/**
 * Keys per `migration/rewrap` batch. A hybrid X25519+ML-KEM wrap is ~1.7 KB of
 * base64, so 500 entries is ~0.85 MB — under the server's migration JSON limit
 * with headroom. Used both as the page size when fetching keys and the chunk
 * size when staging, and matched byte-for-byte by the mobile client so the two
 * emit identical request bodies.
 */
const MIGRATION_REWRAP_BATCH_SIZE = 500

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
    privateKey: string | KeyPair,
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
   * The private material is encrypted with a known device id, so it can be
   * decrypted when the session is refreshed. A legacy account persists its RSA
   * PEM; a curve25519 account persists the whole `v1|ed:|x:` bundle so both the
   * identity and wrapping private keys survive a reload — persisting only
   * `input` would drop the wrapping key and leave file keys unreadable.
   *
   * The downside of this approach is that if someone steals users JWT and refresh
   * token he will be able to decrypt the private material and use it to login.
   *
   * But that requires the attacker to gain access to HTTP only JWT and refresh cookies
   * + to gain access to localStorage where the encrypted private material is stored.
   *
   * This will only be delete out of the browser when user logs out.
   */
  async function setupAndRemember(
    authenticated: Authenticated,
    keypair: KeyPair,
    crypto: CryptoStore
  ) {
    const material = pkBundle.recoveryKeyFor(keypair)

    await pk.setRememberMe(material, authenticated.session.device_id as string)

    return setupAuthenticated(authenticated, keypair, crypto)
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

    const material = await pk.getRememberMe(authenticated.session.device_id as string)
    const keypair = material && (await keyPairFromRememberMe(material, authenticated.user))

    if (keypair) {
      return _withPrivateKey(store, keypair, false)
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

    const material = await pk.getRememberMe(response.body.session?.device_id as string)

    // Remember-me material rebuilds the keypair from scratch (fingerprint
    // verified against the authenticated user); without it we reuse the
    // in-memory keypair, which already carries a curve account's wrapping keys
    // that a bare `input` string would drop.
    const keypair = material
      ? await keyPairFromRememberMe(material, response.body.user)
      : crypto.keypair.input
        ? crypto.keypair
        : null

    if (!keypair) {
      throw new Error(
        'No private key found, please provide your private key when authenticating again'
      )
    }

    await setupAuthenticated(response.body as Authenticated, keypair, crypto)

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
      await setupAndRemember(authenticated, keypair, crypto)
    } else {
      await setupAuthenticated(authenticated, keypair.input as string, crypto)
    }

    logger.info(`[auth] logged in as ${authenticated.user.email}`)

    // Automatic migration for legacy accounts. This is the last time the plaintext
    // password and the decrypted RSA private are both available client-side. The
    // ceremony re-wraps every file key under the new wrapping key, so a failure
    // must never commit a partial re-key — it aborts and the account stays legacy.
    const secVer = authenticated.user.security_version ?? 0
    if (secVer === 0 && credentials.password && credentials.privateKey) {
      try {
        await runMigrationCeremony(authenticated, credentials.privateKey, credentials.password, crypto)
      } catch (e) {
        logger.error('[auth] auto-migration ceremony failed (user stays legacy)', e)
        notify({
          type: 'error',
          title: "Encryption upgrade didn't finish",
          text: "You're signed in and everything works. We'll try upgrading your account again next time you log in."
        })
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
    if (!start.login_id || !start.credential_response || !start.ksf) {
      throw new Error('Invalid opaque login start response')
    }

    const clientFinish = await opaque.clientLoginFinish(
      clientLoginState,
      start.credential_response,
      credentials.password,
      start.ksf
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
    // export_key opens it and yields the Ed identity key plus the wrapping key.
    const exportKeyBytes = cryptfns.uint8.fromBase64(clientFinish.exportKey)

    const kek = await envelope.deriveKek(exportKeyBytes)
    const env = authenticated.user.encrypted_private_key as string
    const bundle = await envelope.open(kek, env)

    const { identity: edPriv, wrapping: xPriv, rsa } = pkBundle.parseBundle(
      new TextDecoder().decode(bundle)
    )

    const kp: KeyPair = {
      input: edPriv || authenticated.user.pubkey || '',
      publicKey: authenticated.user.pubkey || null,
      fingerprint: authenticated.user.fingerprint || null,
      keySize: 0,
      keyType: 'curve25519',
      wrappingPrivate: xPriv || null,
      wrappingPublic: xPriv ? await wrapping.publicFromPrivate(xPriv) : null,
      legacyPrivate: rsa ?? null
    }

    // Record the session and start the refresher, same as the legacy and
    // private-key paths — without this the app never treats an OPAQUE login as
    // authenticated. With Remember Me the curve bundle is persisted so both the
    // identity and wrapping keys survive a reload.
    if (credentials.remember) {
      await setupAndRemember(authenticated, kp, crypto)
    } else {
      await setupAuthenticated(authenticated, kp, crypto)
    }

    logger.info(`[auth] logged in as ${authenticated.user.email} (opaque)`)
    return authenticated as Authenticated
  }

  /**
   * Take a backed-up private key and authenticate with it — no password.
   *
   * A v2 account backs up the curve bundle (`v1|ed:PEM|x:PEM`); a legacy RSA
   * account backs up its RSA PEM. We dispatch on the material: a bundle yields
   * a Curve25519 keypair (identity for signing + wrapping for file keys), an
   * RSA PEM the existing RSA path.
   * @throws
   */
  async function withPrivateKey(
    store: CryptoStore,
    input: PrivateKeyLogin
  ): Promise<Authenticated> {
    const material = input.privateKey || ''

    return _withPrivateKey(store, await keyPairFromMaterial(material), !!input.remember)
  }

  /**
   * Attempt to decrypt the private key and get the current user from backend
   * @throws
   */
  async function withPin(store: CryptoStore, pin: string): Promise<Authenticated> {
    const material = await pk.getPinAndDecrypt(pin)

    return _withPrivateKey(store, await keyPairFromMaterial(material), false)
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
    const curve = keypair.keyType === 'curve25519'

    const fingerprint = curve
      ? await ed25519.fingerprint(keypair.publicKey as string)
      : await cryptfns.rsa.getFingerprint(keypair.input as string)
    const nonce = cryptfns.createFingerprintNonce(fingerprint)
    const signature = curve
      ? await ed25519.sign(nonce, keypair.input as string)
      : await cryptfns.rsa.sign(keypair, nonce)

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
      await setupAndRemember(authenticated, keypair, crypto)
    } else {
      await setupAuthenticated(authenticated, curve ? keypair : (keypair.input as string), crypto)
    }

    logger.info(`[auth] logged in as ${authenticated.user.email} (private key)`)
    return response.body as Authenticated
  }

  function isCurveBundle(material: string): boolean {
    return material.includes('ed:') && material.includes('x:')
  }

  async function curveKeyPairFromBundle(material: string): Promise<KeyPair> {
    const { identity, wrapping: wrappingPriv, rsa } = pkBundle.parseBundle(material)
    if (!identity || !wrappingPriv) {
      throw new Error('Recovery key is missing its identity or wrapping key')
    }
    const publicKey = await ed25519.publicFromPrivate(identity)
    return {
      input: identity,
      publicKey,
      fingerprint: await ed25519.fingerprint(publicKey),
      keySize: 0,
      keyType: 'curve25519',
      wrappingPrivate: wrappingPriv,
      wrappingPublic: await wrapping.publicFromPrivate(wrappingPriv),
      legacyPrivate: rsa ?? null
    }
  }

  /**
   * Rebuild a keypair from stored remember-me material and verify it belongs to
   * the authenticated user. A curve bundle rebuilds both the Ed25519 identity
   * and hybrid wrapping keys; an RSA PEM the legacy keypair. The fingerprint
   * check mirrors the login paths: a mismatch means the stored material is stale
   * or tampered, so we reject it rather than decrypt file metadata with a wrong
   * key.
   */
  async function keyPairFromRememberMe(material: string, user: User): Promise<KeyPair | null> {
    const keypair = await keyPairFromMaterial(material)

    return keypair.fingerprint === user.fingerprint ? keypair : null
  }

  /**
   * Rebuild a KeyPair from backed-up private material — a curve bundle
   * (`v1|ed:|x:`) for a v2 account, an RSA PEM for a legacy one. Every
   * password-less entry point (backup key, PIN unlock, remember-me) routes
   * through here so the curve/RSA dispatch cannot drift between them.
   */
  async function keyPairFromMaterial(material: string): Promise<KeyPair> {
    return isCurveBundle(material)
      ? curveKeyPairFromBundle(material)
      : cryptfns.rsa.inputToKeyPair(material)
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

    // 1. Generate new Ed25519 identity + hybrid wrapping keys
    const newEdPriv = await ed25519.generatePrivateKey()
    const newEdPub = await ed25519.publicFromPrivate(newEdPriv)
    const newXPriv = await wrapping.generatePrivateKey()
    const newXPub = await wrapping.publicFromPrivate(newXPriv)

    // The go-forward fingerprint is the SPKI hash of the new identity key. It
    // must be computed correctly — a wrong value makes the server reject the
    // migration (it recomputes and compares), so there is no safe fallback.
    const newFp = await ed25519.fingerprint(newEdPub)

    // 2. Fetch every key the user holds and re-wrap it under the new wrapping
    // key. A single failure aborts the whole migration: the old RSA key is
    // about to be discarded, so a skipped key would be permanently unreadable.
    // Link keys are wrapped under the owner's key too — an RSA account stored
    // them as an RSA encryption of the link-key hex — so they re-key the same
    // way, otherwise pre-migration links become impossible to unwrap. The key
    // set is paged so the server never holds a whole account's worth at once.
    const rawKeys: MigrationKey[] = []
    const rawLinkKeys: MigrationLinkKey[] = []
    let offset = 0
    for (;;) {
      const page = await Api.get<MigrationKeysResponse>(
        `/api/auth/migration/keys?offset=${offset}&limit=${MIGRATION_REWRAP_BATCH_SIZE}`
      )
      const body = page.body || { keys: [], link_keys: [], next_offset: null }
      rawKeys.push(...body.keys)
      rawLinkKeys.push(...body.link_keys)
      if (body.next_offset == null) break
      offset = body.next_offset
    }

    const rewrapped: MigrationKey[] = []
    let selfCheckSample: { blob: string; fileKey: Uint8Array } | null = null

    for (const k of rawKeys) {
      const fileKeyHex = await cryptfns.rsa.decryptMessage(oldRsaPrivPem, k.encrypted_key)
      const fileKeyBytes = cryptfns.uint8.fromHex(fileKeyHex)
      const newWrapped = await wrapping.wrap(fileKeyBytes, newXPub)
      rewrapped.push({ file_id: k.file_id, encrypted_key: newWrapped })
      if (!selfCheckSample) {
        selfCheckSample = { blob: newWrapped, fileKey: fileKeyBytes }
      }
    }

    const rewrappedLinkKeys: RewrappedLinkKey[] = []
    let linkSelfCheckSample: {
      blob: string
      linkKey: Uint8Array
      fileId: string
      signature: string
    } | null = null

    for (const lk of rawLinkKeys) {
      const linkKeyHex = await cryptfns.rsa.decryptMessage(oldRsaPrivPem, lk.encrypted_link_key)
      const linkKeyBytes = cryptfns.uint8.fromHex(linkKeyHex)
      const newWrapped = await wrapping.wrap(linkKeyBytes, newXPub)
      // Re-sign the link's file_id under the new identity. The stored signature
      // was the owner's old RSA-PSS one; once the account is Ed25519 the server
      // verifies it against the new key, so an un-migrated signature reads as
      // invalid. The canonical is the file UUID string — the same value link
      // creation signs.
      const signature = await ed25519.sign(lk.file_id, newEdPriv)
      rewrappedLinkKeys.push({ link_id: lk.link_id, encrypted_link_key: newWrapped, signature })
      if (!linkSelfCheckSample) {
        linkSelfCheckSample = { blob: newWrapped, linkKey: linkKeyBytes, fileId: lk.file_id, signature }
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

    // The key change is the single most security-relevant event on the owner's
    // audit chain, so it is signed by the new identity and logged in-chain. The
    // server re-encodes this canonical from its own state and aborts the whole
    // migration if the signature is absent or does not verify.
    const auditSignature = await transition.keyRotationAuditSign({
      userId: userIdBytes,
      oldFingerprint: user.fingerprint,
      newFingerprint: newFp,
      rotatedAt: BigInt(issuedAt),
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
    // A re-wrapped key must unwrap under the new wrapping key and match the
    // original — this is what proves every file (and link) survives the re-key.
    if (selfCheckSample) {
      const recovered = await wrapping.unwrap(selfCheckSample.blob, newXPriv)
      const expected = selfCheckSample.fileKey
      const matches =
        recovered.length === expected.length && recovered.every((b, i) => b === expected[i])
      if (!matches) {
        throw new Error('self-check: rewrapped key does not round-trip under the new key')
      }
    }
    if (linkSelfCheckSample) {
      const recovered = await wrapping.unwrap(linkSelfCheckSample.blob, newXPriv)
      const expected = linkSelfCheckSample.linkKey
      const matches =
        recovered.length === expected.length && recovered.every((b, i) => b === expected[i])
      if (!matches) {
        throw new Error('self-check: rewrapped link key does not round-trip under the new key')
      }
      // The re-signature must verify under the new identity, or the server
      // rejects the whole migration with link_signature_invalid.
      const sigOk = await ed25519.verify(
        linkSelfCheckSample.fileId,
        linkSelfCheckSample.signature,
        newEdPub
      )
      if (!sigOk) {
        throw new Error('self-check: rewrapped link signature does not verify under the new key')
      }
    }

    // 7. Stage the re-wrapped keys in batches so no single request body has to
    // carry the whole account's keys. Any failure here throws before complete,
    // so nothing is applied and the staged rows are left for the server to purge
    // — the account stays legacy and the next login retries cleanly.
    const total = rewrapped.length + rewrappedLinkKeys.length
    for (let start = 0; start < total; start += MIGRATION_REWRAP_BATCH_SIZE) {
      const end = Math.min(start + MIGRATION_REWRAP_BATCH_SIZE, total)
      const batchKeys: MigrationKey[] = []
      const batchLinks: RewrappedLinkKey[] = []
      for (let i = start; i < end; i++) {
        if (i < rewrapped.length) batchKeys.push(rewrapped[i])
        else batchLinks.push(rewrappedLinkKeys[i - rewrapped.length])
      }
      await Api.post('/api/auth/migration/rewrap', undefined, {
        keys: batchKeys,
        link_keys: batchLinks
      })
    }

    // 8. Complete the migration: the server applies the staged re-wraps and
    // flips the account in one transaction.
    const completeBody = {
      new_identity_pubkey: newEdPub,
      new_wrapping_pubkey: newXPub,
      new_fingerprint: newFp,
      transition_old_signature: certSigs.oldSignature,
      transition_new_signature: certSigs.newSignature,
      transition_issued_at: issuedAt,
      opaque_registration_upload: regFinish.message,
      encrypted_private_key: env,
      audit_event_signature: auditSignature
    }

    const completeResp = await Api.post('/api/auth/migration/complete', undefined, completeBody)
    if (!completeResp.body) {
      throw new Error('migration/complete failed')
    }

    // 9. Update the in-memory crypto with the new identity (Ed) + wrapping keys.
    // Downstream code (sign, decryptOwn, wrap) will dispatch based on keyType + extra fields.
    const migratedKp: KeyPair = {
      input: newEdPriv,
      publicKey: newEdPub,
      fingerprint: newFp,
      keySize: 0,
      keyType: 'curve25519',
      wrappingPrivate: newXPriv,
      wrappingPublic: newXPub,
      legacyPrivate: oldRsaPrivPem
    }
    await cryptoStore.set(migratedKp)

    migrationNotice.markPending(authenticated.user.id)
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
