import type { User } from './auth';
import * as crypto from './cryptfns/rsa';
import Api from './api';

export interface CreateUser {
	email: string;
	password: string;
	secret?: string;
	token?: string;
	encrypted_secret_key?: string;
	pubkey: string;
	unencrypted_secret_key?: string;
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
	if (!data.encrypted_secret_key && data.unencrypted_secret_key) {
		data.encrypted_secret_key = crypto.encrypt(
			data.unencrypted_secret_key as string,
			data.password as string
		);
	}

	if (!data.unencrypted_secret_key) {
		delete data.unencrypted_secret_key;
	}

	const user = await postRegistration(data as CreateUser);

	return user;
}
