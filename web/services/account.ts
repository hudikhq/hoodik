import type { ChangePassword, UnsecureChangePassword } from 'types'
import Api from '!/api'
import * as cryptfns from '!/cryptfns'

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
