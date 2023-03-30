<script>
	import { onMount } from 'svelte';
	import '../app.postcss';
	import { DarkMode } from 'flowbite-svelte';

	import { ensureAuthenticated } from '$lib/stores';

	onMount(() => {
		ensureAuthenticated();
	});
</script>

<div class="p-8">
	<DarkMode />
	<slot />
</div>
