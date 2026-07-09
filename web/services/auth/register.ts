import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import * as cryptfns from '../cryptfns'
import * as opaque from '../cryptfns/opaque'
import * as envelope from '../cryptfns/envelope'
import * as ed25519 from '../cryptfns/ed25519'
import { encodeBundle } from './bundle'
import { default as Api, type InnerValidationErrors } from '../api'
import type { Authenticated, CreateUser, CryptoStore, KeyPair, LoginStore, User } from 'types'
import type { RouteLocation } from 'vue-router'

interface OpaqueSignupStartRequest {
  email: string
  registration_request: string
}

interface OpaqueSignupStartResponse {
  registration_response: string
}

export const store = defineStore('register', () => {
  const _createUser = ref<CreateUser>({
    email: '',
    password: '',
    pubkey: '',
    fingerprint: '',
    store_private_key: true
  })

  const createUser = computed<CreateUser>(() => _createUser.value as CreateUser)

  const _errors = ref<InnerValidationErrors | null>(null)

  const errors = computed<InnerValidationErrors | null>(
    () => _errors.value as InnerValidationErrors | null
  )

  const _allowRegister = ref<boolean | null>(null)

  const allowRegister = computed<boolean | null>(() => _allowRegister.value)

  /**
   * Public flag the SPA uses to decide whether to render the registration form
   * and the "Create an Account" link on the login pages. Exposed by the server
   * regardless of authentication state — only the boolean is shared, never the
   * whitelist or blacklist rules.
   */
  async function getStatus(): Promise<boolean> {
    if (_allowRegister.value !== null) {
      return _allowRegister.value
    }

    try {
      const response = await Api.get<{ allow_register: boolean }>('/api/auth/register/status')
      _allowRegister.value = response.body?.allow_register ?? true
    } catch {
      // Fall back to allowing the form to render so a transient outage cannot
      // lock new users out. The server still rejects disallowed submissions.
      _allowRegister.value = true
    }

    return _allowRegister.value
  }

  /**
   * This function will be called only from the API response interceptor
   * and error itself always has the unknown type so we need to cast it
   * and accept the unknown type in order to make things easier in components
   */
  function setErrors(errors: InnerValidationErrors | null) {
    _errors.value = errors
  }

  /**
   * Gets the errors
   */
  function getErrors(): InnerValidationErrors | null {
    return _errors.value
  }

  /**
   * Take the route and preload the registration form with the data
   */
  function preload(route: RouteLocation) {
    if (route.query.email) {
      _createUser.value.email = route.query.email as string
    }

    if (route.query.invitation_id) {
      _createUser.value.invitation_id = route.query.invitation_id as string
    }
  }

  /**
   * Set CreateUser object
   */
  async function set(data: Partial<CreateUser>) {
    _createUser.value = { ..._createUser.value, ...data }
  }

  /**
   * Clear CreateUser object and errors
   */
  function clear() {
    _createUser.value = {
      email: '',
      password: '',
      pubkey: '',
      fingerprint: '',
      store_private_key: true
    }
    _errors.value = null
  }

  /**
   * Run the v2 signup ceremony and create the account.
   *
   * A fresh account is born migrated: Curve25519 identity + X25519 wrapping
   * keys, authenticated by OPAQUE. The password never reaches the server —
   * OPAQUE proves it, and its `export_key` seals the private-key bundle into
   * `encrypted_private_key`. Mirrors the migration ceremony in `login.ts`
   * minus the transition certificate (there is no prior key to endorse).
   * @throws
   */
  async function register(
    data: CreateUser,
    login: LoginStore,
    store: CryptoStore
  ): Promise<Authenticated | null> {
    const edPriv = data.identity_private_key as string
    const edPub = data.pubkey
    const xPriv = data.wrapping_private_key as string
    const xPub = data.wrapping_pubkey as string
    const password = data.password as string

    const regStart = await opaque.clientRegistrationStart(password)
    const startResp = await Api.post<OpaqueSignupStartRequest, OpaqueSignupStartResponse>(
      '/api/auth/register/pake/start',
      undefined,
      { email: data.email, registration_request: regStart.message }
    )
    if (!startResp.body) {
      throw new Error('No response from register/pake/start')
    }
    const regFinish = await opaque.clientRegistrationFinish(
      regStart.state,
      startResp.body.registration_response,
      password
    )

    const exportKeyBytes = cryptfns.uint8.fromBase64(regFinish.exportKey)
    const kek = await envelope.deriveKek(exportKeyBytes)
    const bundle = new TextEncoder().encode(encodeBundle({ identity: edPriv, wrapping: xPriv }))
    const env = await envelope.seal(kek, bundle)

    // Prove the sealed bundle reopens and the identity key signs before we
    // commit the account — a broken envelope or key would lock the user out.
    const reopened = await envelope.open(kek, env)
    if (!reopened || reopened.length === 0) {
      throw new Error('self-check: envelope open failed')
    }
    const probe = `register-probe-${Date.now()}`
    const probeSig = await ed25519.sign(probe, edPriv)
    if (!(await ed25519.verify(probe, probeSig, edPub))) {
      throw new Error('self-check: ed25519 signature failed')
    }

    const payload: CreateUser = {
      email: data.email,
      password: '',
      pubkey: edPub,
      wrapping_pubkey: xPub,
      fingerprint: data.fingerprint,
      key_type: 'curve25519',
      encrypted_private_key: env,
      opaque_registration_upload: regFinish.message,
      secret: data.secret,
      token: data.token,
      invitation_id: data.invitation_id
    }
    // The server forbids `password` on a curve account; only OPAQUE proves it.
    delete (payload as Partial<CreateUser>).password

    const response = await Api.post<CreateUser, Authenticated>(
      '/api/auth/register',
      undefined,
      payload
    )
    if (!response.body) {
      return null
    }

    const authenticated = response.body as Authenticated
    const kp: KeyPair = {
      input: edPriv,
      publicKey: edPub,
      fingerprint: data.fingerprint,
      keySize: 0,
      keyType: 'curve25519',
      wrappingPrivate: xPriv,
      wrappingPublic: xPub
    }
    await login.setupAuthenticated(authenticated, kp, store)

    return authenticated
  }

  /**
   * Verify email with provided token from the route params (email)
   * @throws
   */
  async function verifyEmail(token: string): Promise<User> {
    const action = 'activate-email'
    const response = await Api.post(`/api/auth/action/${action}/${token}`)

    return response.body as User
  }

  /**
   * Attempt to resend email verification
   */
  async function resendActivation(email: string): Promise<void> {
    await Api.post(`/api/auth/action/resend`, undefined, { email })
  }

  return {
    createUser,
    set,
    clear,
    register,
    verifyEmail,
    resendActivation,
    preload,

    // Public registration status
    allowRegister,
    getStatus,

    // Errors
    errors,
    setErrors,
    getErrors
  }
})
