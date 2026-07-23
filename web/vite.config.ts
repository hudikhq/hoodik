import fs from 'node:fs'
import path from 'node:path'
import { defineConfig, type Plugin } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'
import { serviceWorkerPlugin } from './plugins/service-worker'

/**
 * Replaces the emitted asset tags — entry script, module preloads and the
 * stylesheet — with a __BOOT_ASSETS__ marker the bootstrap in index.html
 * picks up: each asset's URL and uncompressed size. The bootstrap fetches
 * them all with one combined byte counter — visible on slow connections
 * where the bundle takes ages and a silent screen invites the reload that
 * starts it all over — then reinstates the tags from HTTP cache (assets
 * are content-hashed and served immutable, so nothing downloads twice).
 * Build-only: the dev server serves source modules directly.
 */
const bootProgress = (): Plugin => {
  let outDir = 'dist'
  // The service-worker plugin runs a second Rollup build with the same
  // plugin instances, so a single capture gets clobbered. Remember every
  // emitted file's size and let the written HTML say which ones matter.
  const fileSizes = new Map<string, number>()

  return {
    name: 'hoodik-boot-progress',
    apply: 'build',
    configResolved(config) {
      outDir = config.build.outDir
    },
    generateBundle(_options, bundle) {
      for (const file of Object.values(bundle)) {
        fileSizes.set(
          file.fileName,
          file.type === 'chunk' ? file.code.length : file.source.length
        )
      }
    },
    // The asset tags are appended through Vite's tag-injection mechanism
    // after every HTML transform has run, so a transformIndexHtml hook
    // never sees them — the written file is the first place they exist.
    closeBundle() {
      const htmlPath = path.resolve(outDir, 'index.html')
      if (!fs.existsSync(htmlPath)) return

      const html = fs.readFileSync(htmlPath, 'utf-8')
      const entry = html.match(/<script type="module"[^>]*src="\/([^"]+\.js)"><\/script>\s*/)
      if (!entry) return

      const assets: { url: string; size: number; kind: string }[] = []
      let out = html
      for (const [pattern, kind] of [
        [/<link rel="stylesheet"[^>]*href="\/(assets\/[^"]+\.css)">\s*/g, 'css'],
        [/<link rel="modulepreload"[^>]*href="\/(assets\/[^"]+\.js)">\s*/g, 'preload']
      ] as [RegExp, string][]) {
        for (const tag of out.matchAll(pattern)) {
          assets.push({ url: `/${tag[1]}`, size: fileSizes.get(tag[1]) ?? 0, kind })
        }
        out = out.replace(pattern, '')
      }
      // The crypto WASM dwarfs every script asset and gates mount via
      // top-level await, yet the browser only discovers it after parsing
      // the entry. Fetching it here puts it on the counter and in the HTTP
      // cache before instantiation asks for it.
      for (const [fileName, size] of fileSizes) {
        if (fileName.endsWith('.wasm')) {
          assets.push({ url: `/${fileName}`, size, kind: 'preload' })
        }
      }
      assets.push({ url: `/${entry[1]}`, size: fileSizes.get(entry[1]) ?? 0, kind: 'entry' })

      out = out.replace(
        entry[0],
        `<script>window.__BOOT_ASSETS__=${JSON.stringify(assets)}</script>\n`
      )
      fs.writeFileSync(htmlPath, out)
    }
  }
}

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
    plugins: [wasm(), topLevelAwait()],
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
    bootProgress(),
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
