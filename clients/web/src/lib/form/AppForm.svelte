<script lang="ts">
	// @ts-nocheck
	import { createForm } from './create-form';
	import { setContext } from 'svelte';
	import { key } from './key';
	import { applyAction, enhance } from '$app/forms';
	import { goto } from '$app/navigation';

	export let action = '';
	export let initialValues: any = {};
	export let validate: any = null;
	export let validationSchema: any = null;
	export let onReset: any = () => {
		throw new Error('onReset is a required property in <Form /> when using the fallback context');
	};

	export let context = createForm({
		initialValues,
		onReset,
		validate,
		validationSchema
	});

	const {
		form,
		errors,
		touched,
		state,
		isValid,
		handleChange,
		handleSubmit,
		handleReset,
		updateField,
		updateInitialValues,
		updateTouched,
		updateValidateField,
		validateField
	} = context;

	let values: { [key: string]: unknown } = {};

	form.subscribe((v) => {
		values = v;
	});

	setContext(key, {
		form,
		errors,
		touched,
		state,
		isValid,
		handleChange,
		handleSubmit,
		updateField,
		updateInitialValues,
		updateTouched,
		updateValidateField,
		validateField,
		handleReset
	});

	export const innerSubmitter = ({ form, data, cancel, submitter }) => {
		handleSubmit();

		return async ({ result, update }) => {
			console.debug(result, update);
			applyAction(result);

			console.debug(action);
			goto(action);
		};
	};
</script>

<form class="mt-8 space-y-6" {...$$restProps} method="POST" use:enhance={innerSubmitter}>
	<slot
		{values}
		{form}
		{errors}
		{touched}
		{state}
		{handleChange}
		{handleSubmit}
		{updateField}
		{updateInitialValues}
		{updateTouched}
		{updateValidateField}
		{validateField}
	/>
</form>
