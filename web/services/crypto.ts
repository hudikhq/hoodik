import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import * as rsa from './cryptfns/rsa'
import type { KeyPair } from 'types'

export const store = defineStore('crypto', () => {
  const KeyPair = ref<KeyPair>({
    publicKey: null,
    input: null,
    fingerprint: null,
    keySize: 2048
  })

  const keypair = computed<KeyPair>(() => KeyPair.value)

  /**
   * Set the external keypair value into the internal one.
   * Supports legacy RSA PEMs and (during crypto upgrade) other private key formats (Ed25519 etc).
   * For non-RSA keys we store the raw input so signing/wrapping sites can dispatch by key type.
   */
  async function set(keypair: KeyPair | string) {
    if (typeof keypair === 'string') {
      // Try RSA first for back-compat; if it fails treat as raw new-format key
      try {
        const kp = await rsa.inputToKeyPair(keypair)
        KeyPair.value = kp
        return
      } catch {
        KeyPair.value = { publicKey: null, input: keypair, fingerprint: null, keySize: 0 }
        return
      }
    }

    if (keypair.input) {
      try {
        const kp = await rsa.inputToKeyPair(keypair.input)
        KeyPair.value = { ...kp, ...keypair }
      } catch {
        KeyPair.value = { ...keypair }
      }
    } else if (keypair.publicKey) {
      try {
        const kp = await rsa.publicToKeyPair(keypair.publicKey)
        KeyPair.value = { ...kp, ...keypair }
      } catch {
        KeyPair.value = { ...keypair }
      }
    } else {
      clear()
    }
  }

  /**
   * Clear the keypair value
   */
  async function clear() {
    KeyPair.value = { keySize: 2048, publicKey: null, input: null, fingerprint: null }
  }

  return {
    keypair,
    set,
    clear
  }
})
