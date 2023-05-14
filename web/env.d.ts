/// <reference types="vite/client" />
/// <reference path="cryptfns/cryptfns.d.ts" />
/// <reference types="vite-plugin-pwa/client" />

import type Api from './stores/api'

declare global {
  interface Window {
    UPLOAD: Worker
    UPLOAD_SECOND: Worker
    UPLOAD_THIRD: Worker
    DOWNLOAD: Worker
    CRYPTO: Worker
    SWApi: Api
    canceled: {
      upload: string[]
      download: string[]
    }
  }
}
