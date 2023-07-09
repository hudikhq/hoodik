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
