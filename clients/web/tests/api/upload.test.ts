import { getOrCreate, uploadFile } from './../../src/stores/storage/upload'
import { describe, it, expect } from 'vitest'
import { getUserWithKey } from './register.test'

describe('Upload a file', () => {
  it('API: Can upload a file', async () => {
    await getUserWithKey()

    const data = Buffer.from('Hello, world!')

    const file = await getOrCreate({
      name_enc: 'test.txt',
      encrypted_key: 'some-wanna-be-encrypted-key',
      checksum: '123',
      mime: 'text/plain',
      size: data.length,
      chunks: 1
    })

    expect(!!file).toBeTruthy()

    const uploadedFile = await uploadFile(file, data)

    console.log(uploadedFile)

    expect(!!uploadedFile).toBeTruthy()
    expect(uploadedFile.chunks_stored).toBe(1)
  })
})
