import * as register from '$lib/stores/register';
import * as crypto from '$lib/stores/cryptfns';
import { describe, it, expect } from 'vitest';

const rng = () => `${Math.random() * 99999}`;

export async function getUser(noSecret = false) {
	const keypair = crypto.rsa.generateKey();
	const password = 'some-not-so-weak-password!!1';

	const createUser: register.CreateUser = {
		email: `test-${rng()}@test.com`,
		password,
		pubkey: keypair.publicKey as string,
		fingerprint: await crypto.rsa.getFingerprint(keypair.publicKey as string)
	};

	if (!noSecret) {
		const encrypted = crypto.aes.encrypt(keypair.input as string, createUser.password);
		createUser.encrypted_private_key = encrypted;
	} else {
		createUser.unencrypted_private_key = keypair.input as string;
	}

	const user = await register.register(createUser);

	return { user, password, privateKey: keypair.input as string };
}

describe('API: Register test', () => {
	it('Can we register user', async () => {
		const { user, privateKey, password } = await getUser();

		expect(!!user).toBeTruthy();

		const secret = crypto.aes.decrypt(user.encrypted_private_key as string, password);

		expect(secret).toBe(privateKey);
	});
});
