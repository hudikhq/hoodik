export const CHUNK_SIZE_BYTES = 1024 * 1024 * 4

/**
 * The maximum number of workers to spawn for the crypto jobs
 *
 * This does not include the main two workers that are always running (UPLOAD + DOWNLOAD)
 */
export const MAX_WORKERS = 4

/**
 * Maximum amount of time to wait for a worker to respond
 * before throwing an error
 */
export const MAX_WAIT_FOR_WORKER_MS = 10000

/**
 * Upload constants
 */
export const MAX_UPLOAD_RETRIES = 3
export const CONCURRENT_CHUNKS_UPLOAD = MAX_WORKERS // this is only used when doing it from the worker
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
