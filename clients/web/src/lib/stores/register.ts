import type { AuthenticatedJwt } from './auth';
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

	/**
	 * Optional parameter that is only used in the frontend
	 */
	unencrypted_private_key?: string;
}

/**
 * Make post request to create new user
 * @throws
 */
export async function postRegistration(data: CreateUser): Promise<AuthenticatedJwt> {
	const response = await Api.post<CreateUser, AuthenticatedJwt>(
		'/api/auth/register',
		undefined,
		data
	);

	return response.body as AuthenticatedJwt;
}

/**
 * Generate keypair and register new user
 * @throws
 */
export async function register(data: CreateUser): Promise<AuthenticatedJwt> {
	if (data.unencrypted_private_key) {
		data.encrypted_private_key = crypto.rsa.protectPrivateKey(
			data.unencrypted_private_key as string,
			data.password as string
		);

		// Remove the key from the request payload
		delete data.unencrypted_private_key;
	}

	return postRegistration(data as CreateUser);
}
