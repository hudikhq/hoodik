import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';
import { NodeGlobalsPolyfillPlugin } from '@esbuild-plugins/node-globals-polyfill';
import { resolve } from 'path';
import basicSsl from '@vitejs/plugin-basic-ssl';

export default defineConfig({
	plugins: [basicSsl(), sveltekit()],
	test: {
		environment: 'jsdom',
		include: ['tests/**/*.{test,spec}.{js,ts}']
	},
	envDir: '../../',
	envPrefix: 'APP_',

	optimizeDeps: {
		esbuildOptions: {
			define: {
				global: 'globalThis'
			},
			plugins: [
				NodeGlobalsPolyfillPlugin({
					process: true,
					buffer: true
				})
			]
		}
	},
	resolve: {
		alias: {
			$stores: resolve('./src/stores'),
			constants: 'constants-browserify',
			process: 'process/browser',
			stream: 'stream-browserify',
			zlib: 'browserify-zlib',
			util: 'util',
			crypto: 'crypto-browserify',
			assert: 'assert'
		}
	}
});
