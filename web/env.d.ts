/// <reference types="vite/client" />
/// <reference path="node_modules/transfer/transfer.d.ts" />
/// <reference types="vite-plugin-pwa/client" />

import type Api from './stores/api'

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
