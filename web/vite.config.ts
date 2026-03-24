import path from 'path'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import wasmPack from 'vite-plugin-wasm-pack'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'
import { serviceWorkerPlugin } from './plugins/service-worker'
// import { VitePWA } from 'vite-plugin-pwa'

// wasm-bindgen glue (transfer_bg.js) must not be tree-shaken: WASM imports
// __wbindgen_closure_* from JS; if those exports are dropped, instantiate() fails.
function transferWasmSideEffects(id: string): boolean | undefined {
  if (
    id.includes('transfer_bg.js') ||
    id.includes('transfer_bg.wasm') ||
    id.includes(`${path.sep}transfer${path.sep}pkg`) ||
    id.includes('node_modules/transfer')
  ) {
    return true
  }
  return undefined
}

// https://vitejs.dev/config/
export default defineConfig({
  base: '/',
  envDir: '../',
  envPrefix: 'APP_',
  worker: {
    // Same as main build so nested workers (e.g. hash-worker) resolve `transfer` WASM correctly.
    plugins: [wasm(), topLevelAwait(), wasmPack(['../transfer'])],
    rollupOptions: {
      // Dynamic `import()` inside nested workers (hash-worker) must stay one chunk — otherwise Rollup
      // code-splits and conflicts with the sw.ts worker bundle format (IIFE / service-worker plugin).
      output: {
        inlineDynamicImports: true
      },
      treeshake: {
        moduleSideEffects: transferWasmSideEffects
      }
    }
  },
  plugins: [
    vue(),
    vueJsx(),
    wasm(),
    topLevelAwait(),
    wasmPack(['../transfer']),
    serviceWorkerPlugin({
      filename: 'sw.ts'
    })
  ],
  optimizeDeps: {
    exclude: ['transfer'],
    esbuildOptions: {
      define: {
        global: 'globalThis'
      }
    },
    include: ['vue', 'vue-router', 'pinia']
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
      '!': path.resolve(__dirname, 'services'),
      types: path.resolve(__dirname, 'types')
    }
  },
  css: {
    postcss: {
      plugins: [require('tailwindcss'), require('autoprefixer')]
    }
  },
  build: {
    rollupOptions: {
      treeshake: {
        moduleSideEffects: transferWasmSideEffects
      }
    }
  }
})
