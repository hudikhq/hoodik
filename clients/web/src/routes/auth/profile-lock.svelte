<script lang="ts">
	import { ProfileLock } from '$lib';
	import { Label, Input } from 'flowbite-svelte';
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

<ProfileLock on:submit={onSubmit}>
	<div>
		<Label for="email" class="mb-2">Your email</Label>
		<Input type="email" name="email" id="email" placeholder="name@company.com" required />
	</div>
</ProfileLock>
