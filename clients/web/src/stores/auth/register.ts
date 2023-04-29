import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { store as loginStore } from './login'
import * as crypto from '../cryptfns'
import { default as Api, type InnerValidationErrors } from '../api'
import type { AuthenticatedJwt, CreateUser } from '@/types'

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
    privateKey?: string
  ): Promise<AuthenticatedJwt> {
    const response = await Api.post<CreateUser, AuthenticatedJwt>(
      '/api/auth/register',
      undefined,
      data
    )

    if (privateKey) {
      const login = loginStore()
      login.setupAuthenticated(response.body as AuthenticatedJwt, privateKey)
    }

    return response.body as AuthenticatedJwt
  }

  /**
   * Generate keypair and register new user
   * @throws
   */
  async function register(data: CreateUser): Promise<AuthenticatedJwt> {
    const privateKey = data.unencrypted_private_key

    if (data.unencrypted_private_key && data.store_private_key) {
      data.encrypted_private_key = await crypto.rsa.protectPrivateKey(
        data.unencrypted_private_key as string,
        data.password as string
      )

      // Remove the key from the request payload
      delete data.unencrypted_private_key
    }

    return postRegistration(data, privateKey)
  }

  /**
   * Ask backend to generate two factor secret
   * @throws
   */
  async function getTwoFactorSecret(): Promise<string | null> {
    const response = await Api.get<{ secret: string }>('/api/auth/two-factor-secret')

    return response.body?.secret as string
  }

  return {
    createUser,
    set,
    clear,
    register,
    getTwoFactorSecret,

    // Errors
    errors,
    setErrors,
    getErrors
  }
})
