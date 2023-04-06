import { describe, it, expect } from 'vitest';
import { getUserWithoutKey, getUserWithKey } from './register.test';
import * as auth from '../../src/stores/auth/login';
import * as crypto from '../../src/stores/cryptfns';

describe('Auth test', () => {
	it('API: Can login with credentials', async () => {
		const { user, password } = await getUserWithKey();
		const authenticated = await auth.login({
			email: user.email,
			password
		});
		expect(!!authenticated).toBeTruthy();
		const keypair = await crypto.get();
		expect(keypair.input).toBeTruthy();
	});
	it('API: Can not login with only email and password if the secure way of registering has been done (without encrypted secret on the server)', async () => {
		const { user, password } = await getUserWithoutKey();
		try {
			await auth.login({
				email: user.email,
				password
			});
		} catch (e) {
			expect((e as Error).message).toBe(
				'No private key found, please provide your private key when authenticating'
			);
		}
	});
	it('API: Can login with credentials and privateKey', async () => {
		const { user, password, privateKey } = await getUserWithoutKey();
		const authenticated = await auth.login({
			email: user.email,
			password,
			privateKey
		});
		expect(!!authenticated).toBeTruthy();
		const keypair = await crypto.get();
		expect(keypair.input).toBeTruthy();
	});
	it('API: Can login only with privateKey', async () => {
		const { user, privateKey } = await getUserWithoutKey();
		const authenticated = await auth.loginWithPrivateKey({ privateKey });
		expect(!!authenticated).toBeTruthy();
		expect(authenticated.user.email).toBe(user.email);
		expect(authenticated.user.pubkey).toBe(user.pubkey);
		const keypair = await crypto.get();
		expect(keypair.input).toBeTruthy();
	});
	it('API: Can login only with pin', async () => {
		const { user, privateKey } = await getUserWithoutKey();
		const pin = '123';
		crypto.encryptPrivateKeyAndStore(privateKey, pin);
		const authenticated = await auth.loginWithPin(pin);
		expect(!!authenticated).toBeTruthy();
		expect(authenticated.user.email).toBe(user.email);
		expect(authenticated.user.fingerprint).toBe(user.fingerprint);
		const keypair = await crypto.get();
		expect(keypair.input).toBeTruthy();
	});
});
