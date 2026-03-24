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
    // VitePWA({
    //   registerType: 'autoUpdate',
    //   filename: 'pwa.js',
    //   manifestFilename: 'hoodik.webmanifest',
    //   injectRegister: 'auto',
    //   workbox: {
    //     navigateFallback: '/index.html',
    //     runtimeCaching: [
    //       {
    //         urlPattern: new RegExp('^https://fonts.(?:googleapis|gstatic).com/(.*)'),
    //         handler: 'CacheFirst'
    //       },
    //       {
    //         urlPattern: ({ url }) => {
    //           return url.pathname.startsWith('/assets')
    //         },
    //         handler: 'CacheFirst',
    //         options: {
    //           cacheName: 'assets',
    //           cacheableResponse: {
    //             statuses: [0, 200]
    //           }
    //         }
    //       }
    //     ]
    //   },
    //   manifest: {
    //     name: 'Hoodik - End 2 End Encrypted File Storage',
    //     short_name: 'Hoodik',
    //     icons: [
    //       {
    //         src: '/favicon-16x16.png',
    //         sizes: '16x16',
    //         type: 'image/png'
    //       },
    //       {
    //         src: '/favicon-32x32.png',
    //         sizes: '32x32',
    //         type: 'image/png'
    //       },
    //       {
    //         src: '/apple-touch-icon.png',
    //         sizes: '180x180',
    //         type: 'image/png'
    //       },
    //       {
    //         src: '/android-chrome-192x192.png',
    //         sizes: '192x192',
    //         type: 'image/png'
    //       },
    //       {
    //         src: '/android-chrome-512x512.png',
    //         sizes: '512x512',
    //         type: 'image/png'
    //       }
    //     ],
    //     theme_color: '#A63446',
    //     background_color: '#1E1E1E',
    //     display: 'standalone'
    //   }
    // })
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
