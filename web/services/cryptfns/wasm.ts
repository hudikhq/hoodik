import {
  // default as i,
  crc16_digest,
  sha256_digest,
  rsa_generate_private,
  rsa_public_from_private,
  rsa_decrypt,
  rsa_encrypt,
  rsa_fingerprint_public,
  rsa_fingerprint_private,
  rsa_sign,
  rsa_verify,
  rsa_public_key_size,
  rsa_private_key_size,
  aes_generate_key,
  aes_encrypt,
  aes_decrypt,
  chacha_generate_key,
  chacha_encrypt,
  chacha_decrypt,
  text_into_hashed_tokens
} from '../../node_modules/cryptfns/cryptfns.js'

/**
 * Currently we don't need this part.. it is working fine like this.
 * When we do wasm-pack with --target web it works on the web, but
 * it doesn't work on nodejs. So the tests are breaking which is not
 * acceptable.
 */
// let initialized = false
export async function init() {
  // if (!initialized) {
  //   await i()
  //   initialized = true
  // }
}

export {
  crc16_digest,
  sha256_digest,
  rsa_generate_private,
  rsa_public_from_private,
  rsa_decrypt,
  rsa_encrypt,
  rsa_fingerprint_public,
  rsa_fingerprint_private,
  rsa_sign,
  rsa_verify,
  rsa_public_key_size,
  rsa_private_key_size,
  aes_generate_key,
  aes_encrypt,
  aes_decrypt,
  chacha_generate_key,
  chacha_encrypt,
  chacha_decrypt,
  text_into_hashed_tokens
}
