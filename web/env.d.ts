/// <reference types="vite/client" />
/// <reference path="node_modules/transfer/transfer.d.ts" />
/// <reference types="vite-plugin-pwa/client" />


import type Api from './stores/api'
import type { ComposerTranslation } from 'vue-i18n'

// vue-i18n 9 augments 'vue', which Vue 3.3 does not pick up for template
// type-checking; mirror the augmentation on '@vue/runtime-core' so `$t`
// resolves in SFC templates without per-component imports.
declare module '@vue/runtime-core' {
  interface ComponentCustomProperties {
    $t: ComposerTranslation
  }
}

declare global {
  interface Window {
    __IDENTITY: string | undefined
    defaultDocumentTitle: string
    UPLOAD: Worker
    DOWNLOAD: Worker
    HASH?: Worker
    CRYPTO: Worker
    SWApi: Api
    canceled: {
      upload: string[]
      download: string[]
    }
  }
}
