import { describe, it, expect } from 'vitest';
import { getUser } from './register.test';
import * as auth from '$lib/stores/auth';
import * as crypto from '$lib/stores/cryptfns/rsa';

describe('Auth test', () => {
	it('is a dummy', () => {
		// nothing
	});
	// it('Can login with credentials', async () => {
	// 	const { user, password } = await getUser();
	// 	const authenticated = await auth.login({
	// 		email: user.email,
	// 		password
	// 	});
	// 	expect(!!authenticated).toBeTruthy();
	// 	const keypair = await crypto.get();
	// 	expect(keypair).toBeTruthy();
	// });
	// it('Can not login with only email and password if the secure way of registering has been done (without encrypted secret on the server)', async () => {
	// 	const { user, password } = await getUser(true);
	// 	try {
	// 		await auth.login({
	// 			email: user.email,
	// 			password
	// 		});
	// 	} catch (e) {
	// 		expect((e as Error).message).toBe(
	// 			'No encrypted secret key found on user from backend, not mnemonic provided'
	// 		);
	// 	}
	// });
	// it('Can login with credentials and mnemonic', async () => {
	// 	const { user, password, mnemonic } = await getUser(true);
	// 	const authenticated = await auth.login({
	// 		email: user.email,
	// 		password,
	// 		mnemonic
	// 	});
	// 	expect(!!authenticated).toBeTruthy();
	// 	const keypair = await crypto.get();
	// 	expect(keypair).toBeTruthy();
	// });
	// it('Can login only with mnemonic', async () => {
	// 	const { user, mnemonic } = await getUser(true);
	// 	console.log(user, mnemonic, mnemonic.split(' ').length);
	// 	const authenticated = await auth.loginWithMnemonic(mnemonic);
	// 	expect(!!authenticated).toBeTruthy();
	// 	expect(authenticated.user.email).toBe(user.email);
	// 	expect(authenticated.user.pubkey).toBe(user.pubkey);
	// 	const keypair = await crypto.get();
	// 	expect(keypair).toBeTruthy();
	// });
});
