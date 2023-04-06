<script lang="ts">
	// @ts-nocheck
	import { Button } from 'flowbite-svelte';
	import { getContext } from 'svelte';
	import { key } from './key';

	export let label: string = '';

	const { form } = getContext(key);

	export let disabled: boolean | null = null;
</script>

{#if form}
	<Button {...$$restProps} disabled={disabled === true}>{label}</Button>
{/if}
