import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import * as crypto from '../cryptfns'
import { default as Api, type InnerValidationErrors } from '../api'
import type { Authenticated, CreateUser, CryptoStore, LoginStore, User } from 'types'
import type { RouteLocation } from 'vue-router'

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
   * Make post request to create new user
   * @throws
   */
  async function postRegistration(
    data: CreateUser,
    privateKey: string,
    login: LoginStore,
    store: CryptoStore
  ): Promise<Authenticated | null> {
    const response = await Api.post<CreateUser, Authenticated>(
      '/api/auth/register',
      undefined,
      data
    )

    if (!response.body) {
      return null
    }

    login.setupAuthenticated(response.body as Authenticated, privateKey, store)
    await store.set(privateKey)

    return response.body as Authenticated
  }

  /**
   * Generate keypair and register new user
   * @throws
   */
  async function register(
    data: CreateUser,
    login: LoginStore,
    store: CryptoStore
  ): Promise<Authenticated | null> {
    const privateKey = data.unencrypted_private_key as string

    if (data.unencrypted_private_key) {
      data.encrypted_private_key = await crypto.rsa.protectPrivateKey(
        data.unencrypted_private_key as string,
        data.password as string
      )

      // Remove the key from the request payload
      delete data.unencrypted_private_key
    }

    return postRegistration(data, privateKey, login, store)
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
