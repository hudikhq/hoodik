<script>
	import { A, P, List, Li, Heading } from 'flowbite-svelte';
</script>

<h1 class="text-4xl dark:text-white py-8">Welcome to Flowbite Svelte Admin Dashboard</h1>

<Heading tag="h2" class="my-4" customSize="text-3xl">Pages</Heading>
<List class="space-y-1">
	<Li><A href="/pages/404" class="underline hover:no-underline">404</A></Li>
	<Li><A href="/pages/500" class="underline hover:no-underline">500</A></Li>
	<Li><A href="/pages/maintenance" class="underline hover:no-underline">Maintenance</A></Li>
</List>

<Heading tag="h2" class="my-4" customSize="text-3xl">Authentication</Heading>
<List class="space-y-1">
	<Li><A href="/auth/login" class="underline hover:no-underline">Login</A></Li>
	<Li><A href="/auth/register" class="underline hover:no-underline">Register</A></Li>
</List>
