import { TransferDownloader } from 'transfer'

import type { ApiTransfer } from '../../api'

/**
 * Everything the crate needs to fetch and decrypt one file, in a form that
 * survives `postMessage`.
 *
 * Deliberately not an `AppFile`: a worker has no business receiving a row it
 * might read some other field off, and the fields below are the whole contract.
 */
export interface DownloadSpec {
  id: string
  size: number
  chunks: number
  cipher: string
  key: Uint8Array

  /**
   * Presigned bucket URLs indexed by chunk, when the deployment serves them.
   * An index this does not cover is fetched through the server.
   */
  directUrls?: string[]

  /**
   * Set for a public share link, where `id` is the link id rather than a file
   * id and there is no session to authenticate with.
   */
  publicLink?: boolean
}

/**
 * Build a configured downloader.
 *
 * The one place a `TransferDownloader` is constructed. It was three places,
 * and the copy inside the download worker forgot `set_direct_urls` — so every
 * download in the browser relayed through the server for the whole life of
 * direct transfer while the preview, built by a different copy, did not. A
 * shared factory is what makes that class of drift impossible rather than
 * merely unlikely.
 *
 * Callers must `free()` the result.
 */
export function buildDownloader(spec: DownloadSpec, api: ApiTransfer): TransferDownloader {
  const downloader = spec.publicLink
    ? TransferDownloader.forPublicLink(spec.id, spec.size, spec.chunks, api.apiUrl || '', spec.key)
    : new TransferDownloader(
        spec.id,
        spec.size,
        spec.chunks,
        api.apiUrl || '',
        api.jwtToken || undefined,
        api.refreshToken || undefined,
        spec.key
      )

  downloader.set_cipher(spec.cipher)

  if (spec.directUrls?.length) {
    downloader.set_direct_urls(spec.directUrls)
  }

  return downloader
}
