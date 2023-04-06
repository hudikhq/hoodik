// See https://kit.svelte.dev/docs/types#app
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface Platform {}
	}
}

export {};

interface ImportMetaEnv {
	APP_URL: string;
	APP_CLIENT_URL?: string;
}
