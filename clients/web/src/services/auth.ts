import { writable } from 'svelte/store';

export interface Authenticated {
	user: User;
	session?: Session;
}

export interface User {
	id: number;
	email: string;
	secret?: string;
	pubkey: string;
	created_at: Date;
	updated_at: Date;
}

export interface Session {
	id: number;
	user_id: number;
	token: string;
	csrf: string;
	created_at: Date;
	updated_at: Date;
	expires_at: Date;
}

const authStore = () => {
	const { subscribe, set } = writable<Authenticated | null>();

	const setAuthenticated = (authenticated: Authenticated) => set(authenticated);

	const clearAuthenticated = () => set(null);

	return {
		subscribe,
		setAuthenticated,
		clearAuthenticated
	};
};

export const auth = authStore();
