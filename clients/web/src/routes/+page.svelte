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
	<Li><A href="/auth/profile-lock" class="underline hover:no-underline">Profile lock</A></Li>
	<Li><A href="/auth/reset-password" class="underline hover:no-underline">Reset password</A></Li>
	<Li><A href="/auth/sign-in" class="underline hover:no-underline">Sign in</A></Li>
	<Li><A href="/auth/sign-up" class="underline hover:no-underline">Sign up</A></Li>
</List>
