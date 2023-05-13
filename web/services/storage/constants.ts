export const CHUNK_SIZE_BYTES = 1024 * 1024 * 4

/**
 * Enable or disable offloading the crypto operations while
 * uploading and downloading files (in UPLOAD and DOWNLOAD workers)
 * to the CRYPTO workers.
 *
 * This is useful because the crypto operations are blocking and quite
 * CPU intensive, so this way we can offload them to other workers
 * so they don't block the ones that are waiting to encrypt data.
 */
export const ENABLE_CRYPTO_WORKERS = true

/**
 * The maximum number of workers to spawn for the crypto jobs
 *
 * This does not include the main two workers that are always running (UPLOAD + DOWNLOAD)
 */
export const MAX_CRYPTO_WORKERS = 4

/**
 * Maximum amount of time to wait for a worker to respond
 * before throwing an error
 */
export const MAX_WAIT_FOR_CRYPTO_WORKER_MS = 10000

/**
 * Upload constants
 */
export const MAX_UPLOAD_RETRIES = 3
export const CONCURRENT_CHUNKS_UPLOAD = MAX_CRYPTO_WORKERS // this is only used when doing it from the worker
export const FILES_UPLOADING_AT_ONE_TIME = 1
export const KEEP_FINISHED_UPLOADS_FOR_MINUTES = 15

/**
 * Download constants
 */
export const DOWNLOAD_POOL_LIMIT = 1
export const FILES_DOWNLOADING_AT_ONE_TIME = 1
export const KEEP_FINISHED_DOWNLOADS_FOR_MINUTES = 15

/**
 * Preview constants
 */
export const PREVIEW_MIME_TYPES = ['image/jpg', 'image/jpeg', 'image/png', 'image/gif', 'image/bmp']

export const IMAGE_THUMBNAIL_SIZE_PX = 200
