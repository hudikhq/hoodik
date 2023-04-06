import * as api from './api';
import * as auth from './auth';
import * as crypto from './cryptfns';
export { auth, crypto, api };

/**
 * Takes the date and ensures it is LOCAL time
 *
 * @param {string|Date} [date]
 * @returns {Date}
 */
export function local(date?: string | Date): Date {
	if (!date) {
		date = new Date();
	}

	if (typeof date === 'string') {
		date = new Date(date);
	}

	return new Date(date.getTime() - date.getTimezoneOffset() * 60 * 1000);
}

/**
 * Takes the date and ensures it is UTC time
 *
 * @param {string|Date} [date]
 * @returns {Date}
 */
export function utc(date?: string | Date): Date {
	if (!date) {
		date = new Date();
	}

	if (typeof date === 'string') {
		date = new Date(date);
	}

	return new Date(date.getTime() - date.getTimezoneOffset() * 60 * 1000);
}

export type FormExtractorTemplateType =
	| 'string'
	| 'string[]'
	| 'cString[]'
	| 'number'
	| 'number[]'
	| 'cNumber[]'
	| 'boolean'
	| 'File';

export type FormExtractorValueType =
	| string
	| string[]
	| number
	| number[]
	| boolean
	| File
	| { [key: string]: FormExtractorValueType };

export interface FormExtractorTemplate {
	[key: string]: FormExtractorTemplateType;
}

export interface FormExtractorResult {
	[key: string]: FormExtractorValueType;
}

/**
 * Helper for extracting data from forms
 */
export function formBody(
	data: FormData,
	template: FormExtractorTemplate,
	defaultValues: FormExtractorResult = {}
): FormExtractorResult {
	const result: FormExtractorResult = {};

	for (const key in template) {
		const d = data.get(key);
		let value;

		if (d === null && defaultValues[key] === undefined) {
			continue;
		}

		if (d === null && defaultValues[key] !== undefined) {
			value = defaultValues[key];
			continue;
		}

		if (d) {
			value = d;
		}

		if (value) {
			result[key] = parseValue(value, template[key]) as FormExtractorValueType;
		}
	}

	return result;
}

/**
 * Try to parse whatever kind of value we receive from the form
 */
function parseValue(
	value: FormExtractorValueType,
	type: FormExtractorTemplateType
): FormExtractorValueType | undefined {
	if (type === 'string') {
		return value.toString();
	}

	if (type === 'string[]' && Array.isArray(value)) {
		return value.map((v) => v.toString());
	}

	if (type === 'cString[]') {
		return value.toString().split(',');
	}

	if (type === 'number') {
		return Number(value);
	}

	if (type === 'number[]' && Array.isArray(value)) {
		return value.map((v) => Number(v));
	}

	if (type === 'cNumber[]') {
		return value
			.toString()
			.split(',')
			.map((v) => Number(v));
	}

	if (type === 'boolean') {
		return value.toString() === 'true';
	}

	if (
		type === 'File' &&
		value &&
		typeof value === 'object' &&
		typeof (value as unknown as File).stream === 'function'
	) {
		return value as unknown as File;
	}
}
