import path from 'path'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import wasmPack from 'vite-plugin-wasm-pack'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'
import { serviceWorkerPlugin } from './plugins/service-worker'
import { VitePWA } from 'vite-plugin-pwa'

// https://vitejs.dev/config/
export default defineConfig({
  base: '/',
  envDir: '../',
  envPrefix: 'APP_',
  plugins: [
    vue(),
    vueJsx(),
    wasm(),
    topLevelAwait(),
    wasmPack('../cryptfns'),
    serviceWorkerPlugin({
      filename: 'sw.ts'
    }),
    VitePWA({
      devOptions: {
        enabled: true
      },
      registerType: 'autoUpdate',
      filename: 'pwa.js',
      manifestFilename: 'hoodik.webmanifest',
      injectRegister: 'auto',
      manifest: {
        name: 'Hoodik - End 2 End Encrypted File Storage',
        short_name: 'Hoodik',
        icons: [
          {
            src: '/favicon-16x16.png',
            sizes: '16x16',
            type: 'image/png'
          },
          {
            src: '/favicon-32x32.png',
            sizes: '32x32',
            type: 'image/png'
          },
          {
            src: '/apple-touch-icon.png',
            sizes: '180x180',
            type: 'image/png'
          },
          {
            src: '/android-chrome-192x192.png',
            sizes: '192x192',
            type: 'image/png'
          },
          {
            src: '/android-chrome-512x512.png',
            sizes: '512x512',
            type: 'image/png'
          }
        ],
        theme_color: '#A63446',
        background_color: '#1E1E1E',
        display: 'standalone'
      }
    })
  ],
  optimizeDeps: {
    exclude: ['cryptfns'],
    esbuildOptions: {
      define: {
        global: 'globalThis'
      }
    }
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
  }
})
