import path from 'path'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import wasmPack from 'vite-plugin-wasm-pack'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'
import { serviceWorkerPlugin } from './plugins/service-worker'
// import { VitePWA } from 'vite-plugin-pwa'
import { Key } from './types/cryptfns';

// https://vitejs.dev/config/
export default defineConfig({
  base: '/',
  envDir: '../',
  envPrefix: 'APP_',
  server: process.env.NODE_ENV !== 'production' ? {
    https: {
        key: path.resolve(__dirname, '/data/hoodik.key.pem'),
        cert: path.resolve(__dirname, '/data/hoodik.crt.pem')
      },
    proxy: {
      '/api': {
        target: 'https://127.0.0.1:5443',
        changeOrigin: true,
        secure: false,
        ws: false,
        configure: (proxy, _options) => {
          proxy.on('error', (err, _req, _res) => {
            console.log('proxy error', err);
          });
          proxy.on('proxyReq', (proxyReq, req, _res) => {
            console.log('Sending Request to the Target:', req.method, req.url);
          });
          proxy.on('proxyRes', (proxyRes, req, _res) => {
            console.log('Received Response from the Target:', proxyRes.statusCode, req.url);
          });
        },
      }
    }
  } : {},
  plugins: [
    vue(),
    vueJsx(),
    wasm(),
    topLevelAwait(),
    wasmPack('../cryptfns'),
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
    exclude: ['cryptfns'],
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
  }
})
