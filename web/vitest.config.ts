import { fileURLToPath } from 'node:url'
import { mergeConfig } from 'vite'
import { configDefaults, defineConfig } from 'vitest/config'
import viteConfig from './vite.config'

export default mergeConfig(
  viteConfig,
  defineConfig({
    test: {
      // RSA-2048 keypair generation runs in the transfer WASM and is markedly
      // slower on CI than on dev hardware. The share fan-out suites mint several
      // keypairs per test (these files already run 15-30s locally), so the 5s
      // default times out on CI runners. 60s gives the slowest runner ample
      // headroom without masking a genuine hang.
      testTimeout: 60000,
      hookTimeout: 60000,
      deps: {
        inline: ['transfer']
      },
      setupFiles: ['./vitest.setup.ts'],
      environment: 'jsdom',
      exclude: [...configDefaults.exclude, 'e2e/*'],
      root: fileURLToPath(new URL('./', import.meta.url))
    }
  })
)
