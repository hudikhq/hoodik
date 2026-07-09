import type {
  ActivityQuery,
  ChangePassword,
  KeyPair,
  Paginated,
  Session,
  UnsecureChangePassword
} from 'types'
import Api from '!/api'
import * as cryptfns from '!/cryptfns'
import * as opaque from '!/cryptfns/opaque'
import * as envelope from '!/cryptfns/envelope'
import { encodeBundle } from '!/auth/bundle'

interface OpaqueRegisterStartResponse {
  registration_response: string
}

/**
 * Change the password of a v2 (Curve25519 + OPAQUE) account.
 *
 * The account is authenticated by its current session; the new password is
 * proven by re-running OPAQUE registration client-side. Its `export_key` seals
 * the private-key bundle re-derived from the in-memory keys, and the server
 * commits the new password file and the new envelope together — the old
 * password stops working, the new one opens the same keys.
 * @throws
 */
export async function changePasswordV2(keypair: KeyPair, newPassword: string): Promise<void> {
  if (!keypair.input || !keypair.wrappingPrivate) {
    throw new Error('Missing in-memory keys for password change')
  }

  const regStart = await opaque.clientRegistrationStart(newPassword)
  const startResp = await Api.post<{ registration_request: string }, OpaqueRegisterStartResponse>(
    '/api/auth/pake/register/start',
    undefined,
    { registration_request: regStart.message }
  )
  if (!startResp.body) {
    throw new Error('No response from pake/register/start')
  }

  const regFinish = await opaque.clientRegistrationFinish(
    regStart.state,
    startResp.body.registration_response,
    newPassword
  )

  const kek = await envelope.deriveKek(cryptfns.uint8.fromBase64(regFinish.exportKey))
  const bundle = new TextEncoder().encode(
    encodeBundle({ identity: keypair.input, wrapping: keypair.wrappingPrivate })
  )
  const encrypted_private_key = await envelope.seal(kek, bundle)

  await Api.post<{ registration_upload: string; encrypted_private_key: string }, void>(
    '/api/auth/pake/register/finish',
    undefined,
    { registration_upload: regFinish.message, encrypted_private_key }
  )
}

/**
 * Change the current users password
 */
export async function changePassword(payload: UnsecureChangePassword): Promise<void> {
  const kp = await cryptfns.rsa.inputToKeyPair(payload.private_key)

  if (!kp.input) throw new Error('Invalid private key')

  const encrypted_private_key = await cryptfns.rsa.protectPrivateKey(kp.input, payload.password)

  const data: ChangePassword = {
    encrypted_private_key,
    password: payload.password,
    email: payload.email
  }

  if (payload.token) {
    data.token = payload.token
  }

  if (payload.current_password) {
    data.current_password = payload.current_password
  } else {
    data.signature = await cryptfns.rsa.sign(kp, payload.password)
  }

  await Api.post<ChangePassword, void>('/api/auth/account/change-password', undefined, data)
}

/**
 * Ask backend to generate two factor secret
 * @throws
 */
export async function getTwoFactorSecret(): Promise<string | null> {
  const response = await Api.get<{ secret: string }>('/api/auth/two-factor')

  return response.body?.secret as string
}

/**
 * Disable users two factor authentication
 * @throws
 */
export async function disableTwoFactor(token: string): Promise<void> {
  await Api.post<{ token: string }, void>('/api/auth/two-factor/disable', undefined, { token })
}

/**
 * Enable the users two factor authentication
 * @throws
 */
export async function enableTwoFactor(secret: string, token: string): Promise<void> {
  await Api.post<{ secret: string; token: string }, void>('/api/auth/two-factor', undefined, {
    secret,
    token
  })
}

/**
 * Get paginated array of the sessions sent to the potential new users
 */
export async function activity(query: ActivityQuery): Promise<Paginated<Session>> {
  const response = await Api.get<Paginated<Session>>(`/api/auth/account/activity`, query)

  if (!response.body) {
    throw new Error('Failed to get sessions')
  }

  return response.body
}

/**
 * Kill single user session
 */
export async function kill(id: string): Promise<void> {
  await Api.post<void, void>(`/api/auth/account/kill/${id}`)
}

/**
 * Kill all the users sessions
 */
export async function killAll(): Promise<void> {
  await Api.post<void, void>(`/api/auth/account/kill-all`)
}
