/**
 * Convert base64 text into Uint8Array
 */
export function fromBase64(base64: string) {
  const text = atob(base64)
  const length = text.length
  const bytes = new Uint8Array(length)
  for (let i = 0; i < length; i++) {
    bytes[i] = text.charCodeAt(i)
  }

  return bytes
}

/**
 * Convert Uint8Array into base64 text
 */
export function toBase64(bytes: Uint8Array) {
  let binary = ''
  const len = bytes.byteLength
  for (let i = 0; i < len; i++) {
    binary += String.fromCharCode(bytes[i])
  }

  return btoa(binary)
}

/**
 * Convert utf8 string into Uint8Array
 */
export function fromUtf8(str: string) {
  const bytes = new Uint8Array(str.length)
  for (let i = 0; i < str.length; i++) {
    bytes[i] = str.charCodeAt(i)
  }

  return bytes
}

/**
 * Convert Uint8Array into utf8 string
 */
export function toUtf8(bytes: Uint8Array) {
  return String.fromCharCode.apply(null, bytes as unknown as number[])
}

/**
 * Convert hex string into Uint8Array
 */
export function fromHex(hex: string) {
  const bytes = new Uint8Array(hex.length / 2)
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substr(i, 2), 16)
  }

  return bytes
}

/**
 * Convert Uint8Array into hex string
 */
export function toHex(bytes: Uint8Array) {
  const hex = []
  for (let i = 0; i < bytes.length; i++) {
    hex.push((bytes[i] >>> 4).toString(16))
    hex.push((bytes[i] & 0xf).toString(16))
  }

  return hex.join('')
}
