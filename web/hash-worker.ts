/**
 * Dedicated top-level Web Worker: computes SHA-256 using an incremental
 * (streaming) approach so that arbitrarily large files can be hashed without
 * loading them entirely into memory.
 *
 * Started from main.ts alongside the UPLOAD and DOWNLOAD workers.
 *
 * Protocol:
 *   -> { type: 'hash-file', id: string, file: File }
 *       Read the file in chunks, compute SHA-256, and reply
 *       { type: 'hash-done', id, sha256 } or { type: 'hash-error', id, error }.
 */

/** Read chunk size for hashing: 4 MB. */
const HASH_CHUNK_SIZE = 4 * 1024 * 1024

console.debug('[hash-worker] initialized')

/**
 * Compute SHA-256 of a File by reading it in chunks and hashing each chunk
 * incrementally with SubtleCrypto. Since SubtleCrypto.digest() is
 * non-incremental, we fall back to a manual SHA-256 state machine.
 */
async function hashFile(file: File): Promise<string> {
  // State variables for SHA-256
  const state = new Sha256()

  let offset = 0
  while (offset < file.size) {
    const end = Math.min(offset + HASH_CHUNK_SIZE, file.size)
    const slice = file.slice(offset, end)
    const buffer = await slice.arrayBuffer()
    state.update(new Uint8Array(buffer))
    offset = end
  }

  return state.hexDigest()
}

self.onmessage = async (e: MessageEvent) => {
  const { type, id, file } = e.data as {
    type: string
    id: string
    file: File
  }

  if (type !== 'hash-file') return

  console.debug(`[hash-worker] hashing "${file?.name}" id=${id} size=${file?.size}`)

  try {
    const hex = await hashFile(file)

    console.debug(`[hash-worker] done id=${id} sha256=${hex.slice(0, 8)}...`)
    self.postMessage({ type: 'hash-done', id, sha256: hex })
  } catch (err) {
    console.error('[hash-worker] error:', err)
    self.postMessage({
      type: 'hash-error',
      id,
      error: err instanceof Error ? err.message : String(err)
    })
  }
}

// ---------------------------------------------------------------------------
// Minimal streaming SHA-256 implementation (FIPS 180-4)
//
// We need an incremental hasher because SubtleCrypto.digest() requires the
// entire input at once. This is a straightforward, correct implementation
// optimised for clarity over raw speed — the I/O (file reads) dominates
// the wall-clock time, not the hash computation.
// ---------------------------------------------------------------------------

const K: Uint32Array = new Uint32Array([
  0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
  0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
  0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
  0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
  0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
  0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
  0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
  0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
  0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
  0xc67178f2
])

function rotr(n: number, x: number): number {
  return (x >>> n) | (x << (32 - n))
}

class Sha256 {
  private h: Uint32Array
  private buffer: Uint8Array
  private bufferLen: number
  private totalLen: number
  private w: Uint32Array

  constructor() {
    this.h = new Uint32Array([
      0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
      0x5be0cd19
    ])
    this.buffer = new Uint8Array(64)
    this.bufferLen = 0
    this.totalLen = 0
    this.w = new Uint32Array(64)
  }

  update(data: Uint8Array): void {
    let offset = 0
    this.totalLen += data.length

    // Fill the buffer if there's leftover data from a previous update.
    if (this.bufferLen > 0) {
      const needed = 64 - this.bufferLen
      const toCopy = Math.min(needed, data.length)
      this.buffer.set(data.subarray(0, toCopy), this.bufferLen)
      this.bufferLen += toCopy
      offset = toCopy

      if (this.bufferLen === 64) {
        this.compress(this.buffer, 0)
        this.bufferLen = 0
      }
    }

    // Process full 64-byte blocks directly from the input.
    while (offset + 64 <= data.length) {
      this.compress(data, offset)
      offset += 64
    }

    // Buffer any remaining bytes.
    if (offset < data.length) {
      this.buffer.set(data.subarray(offset), 0)
      this.bufferLen = data.length - offset
    }
  }

  hexDigest(): string {
    // Pad: append 0x80, zero-fill, then append 64-bit big-endian bit length.
    const bitLen = this.totalLen * 8
    const padLen = this.bufferLen < 56 ? 56 - this.bufferLen : 120 - this.bufferLen

    const padding = new Uint8Array(padLen + 8)
    padding[0] = 0x80

    // Write 64-bit big-endian bit length. JavaScript numbers are doubles so
    // files up to 2^53 bytes (~8 PB) are handled correctly.
    const high = Math.floor(bitLen / 0x100000000)
    const low = bitLen >>> 0
    const view = new DataView(padding.buffer)
    view.setUint32(padLen, high, false)
    view.setUint32(padLen + 4, low, false)

    this.update(padding)

    // Convert hash state to hex string.
    const hex: string[] = []
    for (let i = 0; i < 8; i++) {
      hex.push(this.h[i].toString(16).padStart(8, '0'))
    }
    return hex.join('')
  }

  private compress(block: Uint8Array, offset: number): void {
    const w = this.w

    // Prepare message schedule.
    for (let i = 0; i < 16; i++) {
      const j = offset + i * 4
      w[i] =
        (block[j] << 24) | (block[j + 1] << 16) | (block[j + 2] << 8) | block[j + 3]
    }
    for (let i = 16; i < 64; i++) {
      const s0 = rotr(7, w[i - 15]) ^ rotr(18, w[i - 15]) ^ (w[i - 15] >>> 3)
      const s1 = rotr(17, w[i - 2]) ^ rotr(19, w[i - 2]) ^ (w[i - 2] >>> 10)
      w[i] = (w[i - 16] + s0 + w[i - 7] + s1) | 0
    }

    let a = this.h[0]
    let b = this.h[1]
    let c = this.h[2]
    let d = this.h[3]
    let e = this.h[4]
    let f = this.h[5]
    let g = this.h[6]
    let h = this.h[7]

    for (let i = 0; i < 64; i++) {
      const S1 = rotr(6, e) ^ rotr(11, e) ^ rotr(25, e)
      const ch = (e & f) ^ (~e & g)
      const temp1 = (h + S1 + ch + K[i] + w[i]) | 0
      const S0 = rotr(2, a) ^ rotr(13, a) ^ rotr(22, a)
      const maj = (a & b) ^ (a & c) ^ (b & c)
      const temp2 = (S0 + maj) | 0

      h = g
      g = f
      f = e
      e = (d + temp1) | 0
      d = c
      c = b
      b = a
      a = (temp1 + temp2) | 0
    }

    this.h[0] = (this.h[0] + a) | 0
    this.h[1] = (this.h[1] + b) | 0
    this.h[2] = (this.h[2] + c) | 0
    this.h[3] = (this.h[3] + d) | 0
    this.h[4] = (this.h[4] + e) | 0
    this.h[5] = (this.h[5] + f) | 0
    this.h[6] = (this.h[6] + g) | 0
    this.h[7] = (this.h[7] + h) | 0
  }
}
