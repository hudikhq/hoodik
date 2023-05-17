import { describe, it, expect } from 'vitest'
import { rsa } from '../../services/cryptfns'
import { rsa_generate_private, init } from '../../services/cryptfns/wasm'

const privatePem = `-----BEGIN RSA PRIVATE KEY-----
MIIEowIBAAKCAQEAsMvjT2NZNqJo/3AYHH3RIm5fwmOXabbYxduvtNp33JQQZSPu
S+bbqe97jJXVIRUaEPWf05sCwctmFvxcL77FtWLCxaU8TYz4K59LAPcGLGuQO3Hl
PVFAmBUSMDRaK3T3mwMk/Z3qDi8GyumEmN1UfZtxxfqFfMgIB2b8K6enPSHQHoiq
N7xm8MdaDhZGQnyzgjFAKhKFWiusjKwWHe6vvEOFQkIrdTPwhg7ELCmY+kOuW6g/
giE6XcVx2TPOtG5A5qBeJ+vW8XJHZYfCcLwsDZnirTwUTWORWM2omNbquNATh+Lt
3Vh2Rp/vTZJD4LIeR91o55BWr+NLY2I52eSY6QIDAQABAoIBACiQj3Y+qFCV0RuS
36Vh5ONOieAzM6GI15IGRvlrCwdsXZqnNNzrekkybpmiI0W07scnZGWL8oT+o0zw
2EIINprYrzHkKMLubl6r7OyqwRreDzjkeCGqi/SZGRRAXtQLwWgqv4kFe5eHiLpz
+/2LAwDS8rbnNUudJeJ06bUmgYPP5al+96zivJlXhAINss/t5DYwpwoOR7dPaMz0
Yb4qQW9HT5cMi0giDxF89RIV08rFGkDQEU10/31AqqEr8LYMOn/J/JTRqmqnL/DM
kAdwrO1cwU5tnVIgAjj7ouyukDTvxhhk9otrgAwCP6wSOf7V8P4j4AWMHoVYXDTx
fx0JdQUCgYEA2HlCiRLphR/LSil78B+cR3MCJEtn1gGsUei5R+wkw6rjP6UQaTiQ
X4WAAM04B+ZPlQvtnxt/B9+rmM6UcgfPoWLR8RVOp1XMyL/OTwbgljrQc3bWuBiE
OuoYe2QOEtFYg69V4f37+DnHC2gKbbitw62FhaNbZQHNMfC9soh480cCgYEA0RP5
g/l9K/DALiuRE8wkF8ByfRXsYxUGzOu9B5elvAg89wn/LDzsLFfPBZQ/5jml3r99
kvn+eMzavUbqCMKICgD/XZ4cJucoWm+lPtddj/dce27ePVhhWWC7nQ2IzGdswC0T
1O1n6HIa3cMkeruWg36p137E8AVORXxIoOFJSk8CgYBE7pAeYBRWXOJ6Mi2SMC6u
ndPPxOdCwXOi/Y2Kdorad983VBOevfFTSYqSNsch1NgAqTS4lqPj2PimhxnEGfKm
/HXH5DYQmQTF5DYI+jKoBAB+1BfZtYzdyc+T8y98FIewHzQk66DB0XwtiKrRd551
khrTjEo9Js61mWh+onCJXwKBgB0XeW2KpocZrbP+7eXiTtdbONL83PKAd3zGBHxs
9muufcUmB/KA25/j6/NryGRhexn+bRupW2Y1ou4ZUvE7GDDEKMQ+/s3O9kd3J3gS
AXvJwH2QVK4WgR0tn41f17wRXAl1fD/xdLbcQa6/u3C0b2IGmt1YT1DSfCyg+X4h
OtBzAoGBAMqMskbS24GqN21itBVoP+6njQzyXL8r3H3+MHNbhmzuC9MpfE7v4QMI
IKeXOea5ts/4MTXbxUmzFwzj5gFeThsEykJEdHuWK1fbYW2rxFehXH5spqbrzGQX
ejsNMbtGsAhQfwBrKb6qlyNR6d6bTixhCuqYCTYjRi7AnL9G/w9B
-----END RSA PRIVATE KEY-----`

const publicPem = `-----BEGIN RSA PUBLIC KEY-----
MIIBCgKCAQEAsMvjT2NZNqJo/3AYHH3RIm5fwmOXabbYxduvtNp33JQQZSPuS+bb
qe97jJXVIRUaEPWf05sCwctmFvxcL77FtWLCxaU8TYz4K59LAPcGLGuQO3HlPVFA
mBUSMDRaK3T3mwMk/Z3qDi8GyumEmN1UfZtxxfqFfMgIB2b8K6enPSHQHoiqN7xm
8MdaDhZGQnyzgjFAKhKFWiusjKwWHe6vvEOFQkIrdTPwhg7ELCmY+kOuW6g/giE6
XcVx2TPOtG5A5qBeJ+vW8XJHZYfCcLwsDZnirTwUTWORWM2omNbquNATh+Lt3Vh2
Rp/vTZJD4LIeR91o55BWr+NLY2I52eSY6QIDAQAB
-----END RSA PUBLIC KEY-----`

const backendFingerprint = '2faedc21407fe722acb05ff8474417833337675d5e331249fdb09391377b346b'

const encryptedRs =
  'L0/md7W+RFTChUJYV3YfGIwdjWWiPKWe3+98Hk6Z4Yb7zTHY2KzT++RiV1N2nwZyli/w3SjvoXiLpKAk6WTHkMhW8c+fl8A7cZS2Das/bATNsGAFUak2npLCBqqQ49O+EPbrSYjpmlzuEyztyHQ0I/oFWlP0DXkGWyD4QZ8V4fd5JwdSp8KmeM9GJLKREgBA67MHxqHx8sKy6qMByBUmy90chQG1QqrOj5ISl6gkp5t1yr4Cv9SY41mMkS/UPZ5QNaEHM4VO9wQvePA2iLq83whbezuKycrd3teiEojjjUP/qdYe5p4xDcyhYCpT9nPL2Zj9SBv/uMC7T1PHIz+KvA=='

const signatureRs =
  'WHdUZj+VknwKs3A3I9QctDV+ogJ4CToGrC5TRvswzA77xJajhsyEDGxiJJcFtjLnLABdiLBBCnHm6VJY9zx/YH265tFAIofldFQDIwyuF8MVpJHk9ljolm8yrFNhAhRoSrx/6l/7VffEYjIYx9ayD9Db5co5D7Xdk6zpirYJli009jVPt+qWuCdibzgre8QIw8uu3jgGkwdXb2nxfW+lZtXWSOUfaxbk94RYnBu36ojP8oZnNr+SEehhVwX2bOcpiTthtYYsUh/In1+Nlz7H2fZGbDViVHyTvYclw3jXxOksAzRr7t9OJy2/kc4lBMh27SvE8smzapjpTExOv9Is1g=='

const signatureMessage = '28004708'

const encryptionMessage = 'hello world'
describe('Crypto test', () => {
  it('UNIT: WASM: can generate private key from wasm package', async () => {
    await init()
    expect(rsa_generate_private()).toBeTruthy()
  })
  it('UNIT: RSA: can generate secret key from input', async () => {
    const kp = await rsa.generateKeyPair()

    const { input } = await rsa.inputToKeyPair(kp.input as string)

    console.log('PRIVATE:')
    console.log(kp.input)
    console.log('PUBLIC:')
    console.log(kp.publicKey)

    expect(input).toBe(kp.input)

    const encrypted = await rsa.protectPrivateKey(kp.input as string, '123')

    console.log('ENCRYPTED PRIVATE KEY:')
    console.log(encrypted)

    const decrypted = await rsa.decryptPrivateKey(encrypted, '123')

    console.log('DECRYPTED PRIVATE KEY:')
    console.log(decrypted)

    const fingerprint = await rsa.getFingerprint(input as string)
    const fingerprintDecrypted = await rsa.getFingerprint(decrypted as string)

    expect(fingerprintDecrypted).toBe(fingerprint)

    try {
      await rsa.inputToKeyPair(encrypted)
      expect("shouldn't be here").toBeFalsy()
    } catch (e) {
      // should throw
    }
  })

  it('UNIT: RSA: can sign messages and verify signatures', async () => {
    const kp = await rsa.generateKeyPair()

    const message = 'hello world'
    const signature = await rsa.sign(kp, message)

    expect(await rsa.verify(signature, message, kp.publicKey as string)).toBe(true)
  })

  it('UNIT: RSA: can sign messages and verify signatures with specific input', async () => {
    const kp = await rsa.inputToKeyPair(privatePem)

    const kp2 = await rsa.inputToKeyPair(privatePem)

    expect(kp2.publicKey).toBe(kp.publicKey)

    console.log('PRIVATE KEY:')
    console.log(kp.input)
    const privateFingerprint = await rsa.getFingerprint(kp.input as string)
    const publicFingerprint = await rsa.getFingerprint(kp.publicKey as string)

    console.log('PRIVATE KEY:')
    console.log(kp.input)
    console.log('PUBLIC KEY:')
    console.log(kp.publicKey)
    console.log('KEY ID:')
    console.log(publicFingerprint)

    expect(privateFingerprint).toBe(publicFingerprint)
    expect(privateFingerprint).toBe(backendFingerprint)

    const signature = await rsa.sign(kp, signatureMessage)
    console.log('SIGNATURE:')
    console.log(signature)

    expect(await rsa.verify(signature, signatureMessage, publicPem)).toBe(true)
  })

  it('UNIT: RSA: verify signature from the backend', async () => {
    expect(await rsa.verify(signatureRs, signatureMessage, publicPem)).toBe(true)
  })

  it('UNIT: RSA: can encrypt and decrypt message with generated keys', async () => {
    const kp = await rsa.inputToKeyPair(privatePem)

    const encrypted = await rsa.encryptMessage(encryptionMessage, kp.publicKey as string)
    const decrypted = await rsa.decryptMessage(kp, encrypted)

    expect(encryptionMessage !== encrypted).toBe(true)
    expect(decrypted).toEqual(encryptionMessage)
  })

  it('UNIT: RSA: can decrypt message from rust backend', async () => {
    const kp = await rsa.inputToKeyPair(privatePem)

    console.log('KEYSIZE:', kp.keySize)

    const messageBase64 = Buffer.from(encryptionMessage).toString('base64')
    console.log('MESSAGE IN BASE64:', messageBase64)

    expect(encryptionMessage).toEqual(Buffer.from(messageBase64, 'base64').toString())

    const encryptedJs = await rsa.encryptMessage(encryptionMessage, kp.publicKey as string)

    console.log('ENCRYPTED:', encryptedJs)

    const decryptedJs = await rsa.decryptMessage(kp, encryptedJs)

    const decryptedRs = await rsa.decryptMessage(kp, encryptedRs)

    console.log(decryptedRs)
    expect(decryptedRs).toEqual(encryptionMessage)
    expect(decryptedJs).toEqual(encryptionMessage)
  })

  it('UNIT: RSA: can encrypt stuff with new lib', async () => {
    const kp = await rsa.generateKeyPair()

    const encrypted = await rsa.encryptMessage(encryptionMessage, kp.publicKey as string)
    const decrypted = await rsa.decryptMessage(kp, encrypted)

    expect(encryptionMessage !== encrypted).toBe(true)
    expect(decrypted).toEqual(encryptionMessage)
  })

  it('UNIT: RSA: test length of the encryption', async () => {
    const kp = await rsa.generateKeyPair()

    for (let i = 1; i < 245; i++) {
      try {
        const message = 'a'.repeat(i)
        const encrypted = await rsa.encryptMessage(message, kp.publicKey as string)
        const decrypted = await rsa.decryptMessage(kp, encrypted)

        expect(decrypted).toEqual(message)
      } catch (e) {
        expect(`${i} characters`).toBe(false)
      }
    }
  })
})
