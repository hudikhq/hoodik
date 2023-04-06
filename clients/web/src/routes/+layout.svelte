<script>
	import 'constants';
	import { onMount } from 'svelte';
	import { DarkMode } from 'flowbite-svelte';
	import '../app.postcss';

	import { ensureAuthenticated } from '$stores/auth';

	export const ssr = false;

	// @ts-ignore
	console.debug = (...args) => {
		console.log((new Error().stack || '').split('\n').slice(2));
		console.log(...args);
	};

	onMount(async () => {
		// await ensureAuthenticated();
	});
</script>

<div class="p-8">
	<DarkMode />
	<slot />
</div>
