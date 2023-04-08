import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import * as rsa from './cryptfns/rsa'

export const store = defineStore('crypto', () => {
  const KeyPair = ref<rsa.KeyPair>({
    key: null,
    publicKey: null,
    input: null,
    fingerprint: null
  })

  const keypair = computed<rsa.KeyPair>(() => KeyPair.value)

  /**
   * Set the external keypair value into the internal one
   */
  async function set(keypair: rsa.KeyPair) {
    if (keypair.input) {
      const kp = await rsa.inputToKeyPair(keypair.input)
      KeyPair.value = kp
    } else if (keypair.publicKey) {
      const kp = await rsa.publicToKeyPair(keypair.publicKey)
      KeyPair.value = kp
    } else {
      clear()
    }
  }

  /**
   * Clear the keypair value
   */
  async function clear() {
    KeyPair.value = { key: null, publicKey: null, input: null, fingerprint: null }
  }

  return {
    keypair,
    set,
    clear
  }
})
