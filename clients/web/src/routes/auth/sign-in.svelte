<script lang="ts">
	import { Label, Input } from 'flowbite-svelte';
	import { SignIn } from '$lib';

	let title = 'Sign in to platform';
	let site = {
		name: 'Flowbite',
		img: 'https://flowbite-admin-dashboard.vercel.app/images/logo.svg',
		link: 'https://flowbite-admin-dashboard.vercel.app/',
		imgAlt: 'FlowBite Logo'
	};
	let rememberMe = true;
	let lostPassword = true;
	let createAccount = true;
	let lostPasswordLink = 'forgot-password';
	let loginTitle = 'Login to your account';
	let registerLink = 'sign-up';
	let createAccountTitle = 'Create account';

	const onSubmit = (e: Event) => {
		const formData = new FormData(e.target as HTMLFormElement);

		const data: Record<string, string | File> = {};
		for (const field of formData.entries()) {
			const [key, value] = field;
			data[key] = value;
		}
		console.log(data);
	};
</script>

<SignIn
	{title}
	{site}
	{rememberMe}
	{lostPassword}
	{createAccount}
	{lostPasswordLink}
	{loginTitle}
	{registerLink}
	{createAccountTitle}
	on:submit={onSubmit}
>
	<div>
		<Label for="email" class="mb-2">Your email</Label>
		<Input type="email" name="email" id="email" placeholder="name@company.com" required />
	</div>
	<div>
		<Label for="password" class="mb-2">Your password</Label>
		<Input type="password" name="password" id="password" placeholder="••••••••" required />
	</div>
</SignIn>
