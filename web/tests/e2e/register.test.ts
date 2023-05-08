import { createPinia } from 'pinia'
import { store as registerStore } from '../../services/auth/register'
import * as crypto from '../../services/cryptfns'
import { describe, it, expect } from 'vitest'
import { CreateUser } from '../../types'

const pinia = createPinia()
const register = registerStore(pinia)
const rng = () => `${Math.random() * 99999}`

async function getUser(sendKey = false) {
  const keypair = await crypto.rsa.generateKeyPair()
  const password = 'some-not-so-weak-password!!1'

  const createUser: CreateUser = {
    email: `tibor+${rng()}@hudik.eu`,
    password,
    pubkey: keypair.publicKey as string,
    fingerprint: await crypto.rsa.getFingerprint(keypair.publicKey as string)
  }

  if (sendKey) {
    createUser.store_private_key = true
    createUser.unencrypted_private_key = keypair.input as string
  }

  const {
    authenticated: { user },
    jwt
  } = await register.register(createUser)

  return { user, jwt, password, privateKey: keypair.input as string }
}

export async function getUserWithKey() {
  return getUser(true)
}

export async function getUserWithoutKey() {
  return getUser(false)
}

describe('Register test', () => {
  it('E2E: Can we register user', async () => {
    const { user, privateKey, password } = await getUserWithKey()

    expect(!!user).toBeTruthy()

    const secret = await crypto.rsa.decryptPrivateKey(
      user.encrypted_private_key as string,
      password
    )

    const secretFingerprint = await crypto.rsa.getFingerprint(secret)
    const privateKeyFingerprint = await crypto.rsa.getFingerprint(privateKey)

    expect(secretFingerprint).toBe(privateKeyFingerprint)
  })
})
