import dotenv from 'dotenv';

dotenv.config({
	path: '../../.env'
});

export const env = process.env;
