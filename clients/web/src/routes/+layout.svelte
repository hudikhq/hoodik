<script>
	import '../app.postcss';
	import { DarkMode } from 'flowbite-svelte';
</script>

<div class="p-8">
	<div>
		<p class="mb-2 dark:text-white">
			Client base url: {import.meta.env.APP_CLIENT_URL}, API base url: {import.meta.env.APP_API_URL}
		</p>
	</div>

	<DarkMode />
	<slot />
</div>
