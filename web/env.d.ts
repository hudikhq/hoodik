/// <reference types="vite/client" />
/// <reference path="cryptfns/cryptfns.d.ts" />

import type Api from './stores/api'

declare global {
  interface Window {
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
