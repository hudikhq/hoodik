// authentication
export { default as Login } from './authentication/Login.svelte';
export { default as Pin } from './authentication/Pin.svelte';
export { default as Register } from './authentication/Register.svelte';

// pages
export { default as Maintenance } from './pages/Maintenance.svelte';
export { default as NotFound } from './pages/NotFound.svelte';
export { default as ServerError } from './pages/ServerError.svelte';

/**
 * Takes the date and ensures it is LOCAL time
 *
 * @param {string|Date} [date]
 * @returns {Date}
 */
export function local(date?: string | Date): Date {
	if (!date) {
		date = new Date();
	}

	if (typeof date === 'string') {
		date = new Date(date);
	}

	return new Date(date.getTime() - date.getTimezoneOffset() * 60 * 1000);
}

/**
 * Takes the date and ensures it is UTC time
 *
 * @param {string|Date} [date]
 * @returns {Date}
 */
export function utc(date?: string | Date): Date {
	if (!date) {
		date = new Date();
	}

	if (typeof date === 'string') {
		date = new Date(date);
	}

	return new Date(date.getTime() - date.getTimezoneOffset() * 60 * 1000);
}
