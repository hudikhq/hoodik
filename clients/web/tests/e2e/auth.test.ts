import { describe, it, expect } from 'vitest'
import { getUserWithoutKey, getUserWithKey } from './register.test'
import { store as loginStore } from '../../src/stores/auth/login'
import { store as cryptoStore } from '../../src/stores/crypto'
import * as cryptofns from '../../src/stores/cryptfns'
import { createPinia } from 'pinia'

const pinia = createPinia()
const auth = loginStore(pinia)
const crypto = cryptoStore(pinia)

describe('Auth test', () => {
  it('E2E: Can login with credentials', async () => {
    const { user, password } = await getUserWithKey()
    const authenticated = await auth.withCredentials(crypto, {
      email: user.email,
      password
    })
    expect(!!authenticated).toBeTruthy()
    const keypair = crypto.keypair
    expect(keypair.input).toBeTruthy()
  })
  it('E2E: Can not login with only email and password if the secure way of registering has been done (without encrypted secret on the server)', async () => {
    const { user, password } = await getUserWithoutKey()
    try {
      await auth.withCredentials(crypto, {
        email: user.email,
        password
      })
    } catch (e) {
      expect((e as Error).message).toBe(
        'No private key found, please provide your private key when authenticating'
      )
    }
  })
  it('E2E: Can login with credentials and privateKey', async () => {
    const { user, password, privateKey } = await getUserWithoutKey()
    const authenticated = await auth.withCredentials(crypto, {
      email: user.email,
      password,
      privateKey
    })
    expect(!!authenticated).toBeTruthy()
    const keypair = crypto.keypair
    expect(keypair.input).toBeTruthy()
  })
  it('E2E: Can login only with privateKey', async () => {
    const { user, privateKey } = await getUserWithoutKey()
    const authenticated = await auth.withPrivateKey(crypto, { privateKey })
    expect(!!authenticated).toBeTruthy()
    expect(authenticated.user.email).toBe(user.email)
    expect(authenticated.user.pubkey).toBe(user.pubkey)
    const keypair = crypto.keypair
    expect(keypair.input).toBeTruthy()
  })
  it('E2E: Can login only with pin', async () => {
    const { user, privateKey } = await getUserWithoutKey()
    const pin = '123'
    cryptofns.encryptPrivateKeyAndStore(privateKey, pin)
    const authenticated = await auth.withPin(crypto, pin)
    expect(!!authenticated).toBeTruthy()
    expect(authenticated.user.email).toBe(user.email)
    expect(authenticated.user.fingerprint).toBe(user.fingerprint)
    const keypair = crypto.keypair
    expect(keypair.input).toBeTruthy()
  })
})
