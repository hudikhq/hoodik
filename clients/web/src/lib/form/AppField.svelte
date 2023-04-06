<script lang="ts">
	// @ts-nocheck
	import { Label, Input, Button } from 'flowbite-svelte';
	import { getContext } from 'svelte';
	import { key } from './key';
	import { copy } from 'svelte-copy';

	export let name;
	export let type = 'text';
	export let label = '';

	const { form, handleChange, errors } = getContext(key);

	let error = null;
	export let allowCopy: boolean = false;

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
	<div class="w-full items-start">
		{#if label}
			<div class="float-left w-1/2">
				<Label for={name} class="mb-1">
					{label}
				</Label>
			</div>
		{/if}

		{#if allowCopy}
			<div class="float-right w-1/2">
				<button
					class="float-right text-center justify-center text-xs text-gray-700 dark:text-gray-400"
					use:copy={$form[name]}
				>
					Copy to clipboard
				</button>
			</div>
		{/if}
	</div>

	<Input
		bind:value={$form[name]}
		on:change={handleChange}
		on:blur={handleChange}
		{type}
		{name}
		{...$$restProps}
	/>

	{#if error}
		<small class="text-sm text-red-700 dark:text-red-500 block">{error}</small>
	{/if}
</div>
