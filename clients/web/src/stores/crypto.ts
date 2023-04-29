import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import * as rsa from './cryptfns/rsa'
import type { KeyPair } from '@/types'

export const store = defineStore('crypto', () => {
  const KeyPair = ref<KeyPair>({
    publicKey: null,
    input: null,
    fingerprint: null,
    keySize: 2048
  })

  const keypair = computed<KeyPair>(() => KeyPair.value)

  /**
   * Set the external keypair value into the internal one
   */
  async function set(keypair: KeyPair) {
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
    KeyPair.value = { keySize: 2048, publicKey: null, input: null, fingerprint: null }
  }

  return {
    keypair,
    set,
    clear
  }
})
