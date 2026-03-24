import { defineConfig } from '@playwright/test'
import { config } from 'dotenv'
import { resolve } from 'path'

const envFile = process.env.ENV_FILE
  ? resolve(process.env.ENV_FILE)
  : resolve('../.env')

const env = config({ path: envFile }).parsed || {}
// Process env vars take priority so the Justfile can override .env.e2e values
// (e.g. forcing http:// when SSL_DISABLED=true is injected by the e2e recipe).
const baseURL =
  process.env.APP_CLIENT_URL ||
  process.env.APP_URL ||
  env.APP_CLIENT_URL ||
  env.APP_URL ||
  'http://localhost:5173'

export default defineConfig({
  testDir: './e2e',
  fullyParallel: false,
  workers: 1,
  timeout: 60_000,
  retries: 0,
  reporter: [['html', { open: 'never' }], ['list']],
  use: {
    baseURL,
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    viewport: { width: 1280, height: 800 },
  },
})
