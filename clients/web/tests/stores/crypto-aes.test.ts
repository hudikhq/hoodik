import { describe, it, expect, assert } from 'vitest';
import * as crypto from '../../src/lib/stores/cryptfns';

describe('Crypto test', () => {
	it('AES: can encrypt and decrypt string with provided pin using AES', () => {
		const secret = 'little secret';
		const pin = '123456';
		const encrypted = crypto.aes.encryptSecret(secret, pin);

		assert(encrypted !== secret, 'Encrypted value is the same as provided value');

		const decrypted = crypto.aes.decryptSecret(encrypted, pin);

		expect(decrypted).toBe(secret);
	});
});
