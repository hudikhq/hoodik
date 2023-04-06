<script lang="ts">
	// @ts-nocheck
	import { Label, Checkbox } from 'flowbite-svelte';
	import { getContext } from 'svelte';
	import { key } from './key';

	export let name: any;
	export let label: any = '';

	const { form, handleChange, errors } = getContext(key);

	let error = null;
	errors.subscribe((errors) => {
		if (errors && typeof errors === 'object' && errors[name]) {
			error = errors[name];
		} else if (errors && typeof errors === 'string') {
			error = errors;
		} else {
			error = null;
		}
	});
</script>

<div>
	<div class="flex items-start">
		<div class="flex items-center h-5">
			<!-- svelte-ignore ts -->
			<Checkbox
				bind:checked={$form[name]}
				on:change={handleChange}
				on:blur={handleChange}
				{name}
				{...$$props}
			/>
		</div>
		{#if label}
			<div class="ml-3 text-sm">
				<Label for={name}>{label}</Label>
			</div>
		{/if}
	</div>

	{#if error}
		<div class="flex items-start ml-7">
			<small class="text-sm text-red-700 dark:text-red-500">{error}</small>
		</div>
	{/if}
</div>
