import path from 'path'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import { NodeGlobalsPolyfillPlugin } from '@esbuild-plugins/node-globals-polyfill'
import { comlink } from 'vite-plugin-comlink'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'
import wasmPack from './wasm-pack'

// https://vitejs.dev/config/
export default defineConfig({
  base: '/',
  envDir: '../../',
  envPrefix: 'APP_',
  plugins: [
    vue(),
    vueJsx(),
    NodeGlobalsPolyfillPlugin({
      process: true,
      buffer: true
    }),
    wasm(),
    topLevelAwait(),
    wasmPack('../../cryptfns'),
    comlink()
  ],
  optimizeDeps: {
    esbuildOptions: {
      // Node.js global to browser globalThis
      define: {
        global: 'globalThis'
      },
      // Enable esbuild polyfill plugins
      plugins: [
        NodeGlobalsPolyfillPlugin({
          buffer: true
        })
      ]
    }
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
      constants: 'constants-browserify',
      process: 'process/browser',
      stream: 'stream-browserify',
      zlib: 'browserify-zlib',
      util: 'util',
      crypto: 'crypto-browserify',
      assert: 'assert',
      buffer: 'buffer',
      Buffer: 'buffer/Buffer'
    }
  },
  css: {
    postcss: {
      plugins: [require('tailwindcss'), require('autoprefixer')]
    }
  }
})
