import * as cryptfns from './../../src/stores/cryptfns'
import * as storage from './../../src/stores/storage'
import { describe, expect, it } from 'vitest'
import { getUserWithKey } from './register.test'
import { CreateFile } from '../../src/stores/storage/meta'
import { CHUNK_SIZE_BYTES } from '../../src/stores/storage/upload'
import { utcStringFromLocal } from '../../src/stores'

describe('Upload a file', () => {
  it('API: Can upload a file', async () => {
    const { privateKey } = await getUserWithKey()
    const KeyPair = await cryptfns.rsa.inputToKeyPair(privateKey)

    let text = ''
    for (let i = 0; i < CHUNK_SIZE_BYTES * 3; i++) {
      text += '1'
    }

    // @ts-ignore
    const mockFile = new File([text], 'test.txt', {
      type: 'text/plain',
      // @ts-ignore
      size: text.length,
      lastModified: Math.ceil(new Date().getTime() / 1000)
    })

    const modified = mockFile.lastModified ? new Date(mockFile.lastModified) : new Date()

    const createFile: CreateFile = {
      name: mockFile.name,
      size: mockFile.size,
      mime: mockFile.type || 'application/octet-stream',
      chunks: Math.ceil(mockFile.size / CHUNK_SIZE_BYTES),
      file_created_at: utcStringFromLocal(modified)
    }

    let file = await storage.meta.create(KeyPair, createFile)

    await storage.upload.upload({ ...file, file: mockFile }, (f, done) => {
      console.log(
        `Running progress for a file: ${f.metadata?.name || 'unknown - no metadata'} done: ${done}`
      )
      file = f
    })

    const downloadedFile = await storage.download.get(file.id, KeyPair)

    const decoder = new TextDecoder()
    const data = decoder.decode(downloadedFile.data)

    expect(data).toBe(text)
  }, 60000)
})
