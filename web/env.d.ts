/// <reference types="vite/client" />
/// <reference path="cryptfns/cryptfns.d.ts" />
/// <reference types="vite-plugin-pwa/client" />

import type Api from './stores/api'

declare global {
  interface Window {
    __IDENTITY: string | undefined
    UPLOAD: Worker
    DOWNLOAD: Worker
    CRYPTO: Worker
    SWApi: Api
    canceled: {
      upload: string[]
      download: string[]
    }
  }
}
