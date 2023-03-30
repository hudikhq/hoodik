<script lang="ts">
	import { Label, Input } from 'flowbite-svelte';
	import { SignUp } from '$lib';
	const title = 'Create a Free Account';
	const site = {
		name: 'Flowbite',
		img: 'https://flowbite-admin-dashboard.vercel.app/images/logo.svg',
		link: 'https://flowbite-admin-dashboard.vercel.app/',
		imgAlt: 'FlowBite Logo'
	};
	const acceptTerms = true;
	const haveAccount = true;
	const btnTitle = 'Create account';
	const termsLink = '/';
	const loginLink = 'sign-in';
	const labelClass = 'mb-2';
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

<SignUp
	{title}
	{site}
	{acceptTerms}
	{haveAccount}
	{btnTitle}
	{termsLink}
	{loginLink}
	on:submit={onSubmit}
>
	<div>
		<Label for="email" class="mb-2">Your email</Label>
		<Input type="email" name="email" id="email" placeholder="name@company.com" required />
	</div>
	<div>
		<Label for="password" class={labelClass}>Your password</Label>
		<Input type="password" name="password" id="password" placeholder="••••••••" required />
	</div>
	<div>
		<Label for="confirm-password" class={labelClass}>Confirm password</Label>
		<Input
			type="password"
			name="confirm-password"
			id="confirm-password"
			placeholder="••••••••"
			required
		/>
	</div>
</SignUp>
