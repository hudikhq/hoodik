import { vi } from 'vitest'
import { config } from '@vue/test-utils'
import { i18n } from './src/i18n'

// Components resolve `$t` through the app-level i18n plugin; mounts in tests
// need the same plugin or every template using a translation would throw.
config.global.plugins = [...(config.global.plugins ?? []), i18n]

// vue-clipboard3 ships a CJS bundle inside a `type: module` package. Node
// refuses to evaluate the CJS file as ESM, so any AppField mount in jsdom
// blows up before the test starts. Stub the module here — clipboard support
// is irrelevant to the share-feature tests and the API surface is tiny.
vi.mock('vue-clipboard3', () => ({
  default: () => ({
    toClipboard: async () => undefined
  })
}))
