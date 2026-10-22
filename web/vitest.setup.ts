import { vi } from 'vitest'
import { config } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { i18n } from './src/i18n'

// The jsdom environment aliases the typed-array globals to jsdom's window,
// and since jsdom 27 that window is its own realm: a `new Uint8Array()` in a
// test no longer matches the Node-realm bytes TextEncoder produces, so
// `toEqual` and `instanceof` fail across the seam. A browser has one realm;
// give the tests one too. structuredClone allocates in the realm it was
// defined in, which is Node's, so cloning a foreign array yields the native
// constructor.
for (const name of [
  'ArrayBuffer',
  'Uint8Array',
  'Uint8ClampedArray',
  'Uint16Array',
  'Uint32Array',
  'Int8Array',
  'Int16Array',
  'Int32Array',
  'Float32Array',
  'Float64Array'
] as const) {
  globalThis[name] = structuredClone(new globalThis[name](0)).constructor as never
}

// Several services resolve their store at module scope, so importing one from
// a test file needs an active Pinia before the imports run. Suites that care
// about isolation still install their own in `beforeEach`.
setActivePinia(createPinia())

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
