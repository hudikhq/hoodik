import type { User } from './auth';
import * as crypto from './cryptfns';
import Api from './api';

export interface CreateUser {
	email: string;
	password: string;
	secret?: string;
	token?: string;
	pubkey: string;
	fingerprint: string;
	encrypted_private_key?: string;
	unencrypted_private_key?: string;
}

/**
 * Make post request to create new user
 * @throws
 */
export async function postRegistration(data: CreateUser): Promise<User> {
	const response = await Api.post<CreateUser, User>('/api/auth/register', undefined, data);

	return response.body as User;
}

/**
 * Generate keypair and register new user
 * @throws
 */
export async function register(data: CreateUser): Promise<User> {
	if (!data.encrypted_private_key && data.unencrypted_private_key) {
		data.encrypted_private_key = crypto.aes.encrypt(
			data.unencrypted_private_key as string,
			data.password as string
		);
	}

	if (data.unencrypted_private_key) {
		delete data.unencrypted_private_key;
	}

	return postRegistration(data as CreateUser);
}
