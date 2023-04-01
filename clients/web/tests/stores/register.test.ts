import * as register from '$lib/stores/register';
import * as crypto from '$lib/stores/crypto';
import { describe, it, expect } from 'vitest';

const rng = () => `${Math.random() * 99999}`;

export async function getUser(noSecret = false) {
	const keypair = crypto.generateKey();
	const encrypted = crypto.encrypt(keypair.secretKey as string, 'some-not-so-weak-password!!1');

	const user = await register.register({
		email: `test-${rng()}@test.com`,
		password: 'some-not-so-weak-password!!1',
		encrypted_secret_key: noSecret ? undefined : encrypted,
		pubkey: keypair.publicKey as string
	});

	return { user, password: 'some-not-so-weak-password!!1', mnemonic: keypair.secretKey as string };
}

describe('Register test', () => {
	it('Can we register user', async () => {
		// const { user } = await getUser();
		// expect(!!user).toBeTruthy();
	});
});
