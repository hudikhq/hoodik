import { describe, it, expect, assert } from 'vitest';
import * as crypto from '../../src/lib/stores/crypto';

describe('Crypto test', () => {
	it('can encrypt and decrypt string with provided pin using AES', () => {
		const secret = 'little secret';
		const pin = '123456';
		const encrypted = crypto.encrypt(secret, pin);

		assert(encrypted !== secret, 'Encrypted value is the same as provided value');

		const decrypted = crypto.decrypt(encrypted, pin);

		expect(decrypted).toBe(secret);
	});

	it('can generate secret key from mnemonic', async () => {
		const keypair = crypto.generateKey();

		const { secretKey } = crypto.generateKeyFrom(keypair.secretKey as string);

		expect(secretKey).toBe(keypair.secretKey);
	});

	it('can sign messages and verify signatures', async () => {
		const keypair = crypto.generateKey();
		crypto.set(keypair);

		const message = 'hello world';
		const { signature, pubkey } = await crypto.sign(message);

		expect(pubkey).toBe(keypair.publicKey);
		expect(await crypto.verify(signature, message)).toBe(true);
	});
});
