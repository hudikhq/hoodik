import * as lscache from 'lscache'
import * as cryptfns from '../cryptfns'

const REMEMBER_ME_PRIVATE_KEY = 'REMEMBER_ME_PRIVATE_KEY'
const PIN_SAVE_PRIVATE_KEY = 'PIN_SAVE_PRIVATE_KEY'
const PIN_SAVE_PRIVATE_KEY_EMAIL = 'PIN_SAVE_PRIVATE_KEY_EMAIL'

/**
 * Clear the private key stored for the remember me feature
 */
export function clearRememberMe() {
  if (hasRememberMe()) {
    lscache.remove(REMEMBER_ME_PRIVATE_KEY)
  }
}

/**
 * Do we have the private key stored for the remember me feature?
 */
export function hasRememberMe() {
  return !!lscache.get(REMEMBER_ME_PRIVATE_KEY)
}

/**
 * Get the private key stored for remember me and decrypt it
 */
export async function getRememberMe(deviceId: string, fingerprint: string): Promise<string | null> {
  const encrypted = lscache.get(REMEMBER_ME_PRIVATE_KEY) || null

  if (!encrypted) {
    return null
  }

  const privateKey = await cryptfns.aes.decryptString(encrypted, deviceId)

  if (!privateKey) {
    clearRememberMe()
    return null
  }

  const fp = await cryptfns.rsa.getFingerprint(privateKey)

  if (fp !== fingerprint) {
    clearRememberMe()
    return null
  }

  return privateKey
}

/**
 * Encrypt and set remember me key in localStorage
 */
export async function setRememberMe(privateKey: string, deviceId: string) {
  lscache.set(REMEMBER_ME_PRIVATE_KEY, await cryptfns.aes.encryptString(privateKey, deviceId))
}

/**
 * Get email of the user who has stored private key and encrypted it with a pin
 */
export function getPinEmail(): string | null {
  return lscache.get(PIN_SAVE_PRIVATE_KEY_EMAIL)
}

/**
 * Get private key encrypted with the pin
 */
export function getPin(): string | null {
  return lscache.get(PIN_SAVE_PRIVATE_KEY)
}

/**
 * Do we have private key encrypted with the pin?
 */
export function hasPin(): boolean {
  return !!getPin()
}

/**
 * Remove the private key that was encrypted with a pin (for quick login)
 */
export function clearPin(): void {
  lscache.remove(PIN_SAVE_PRIVATE_KEY)
  lscache.remove(PIN_SAVE_PRIVATE_KEY_EMAIL)
}

/**
 * Encrypt private key with pin and store it in localStorage
 */
export async function pinEncryptAndStore(pk: string, pin: string, email: string) {
  const encrypted = await cryptfns.aes.encryptString(pk, pin)

  lscache.set(PIN_SAVE_PRIVATE_KEY, encrypted)
  lscache.set(PIN_SAVE_PRIVATE_KEY_EMAIL, email)
  clearRememberMe()
}

/**
 * Get the pin encrypted private key from localStorage and decrypt it
 */
export async function getPinAndDecrypt(pin: string): Promise<string> {
  const pk = getPin()

  if (!pk) {
    throw new Error('No encrypted private key found')
  }

  return cryptfns.aes.decryptString(pk, pin)
}
