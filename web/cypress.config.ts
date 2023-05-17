import { defineConfig } from 'cypress'
import * as fs from 'fs'
import { resolve } from 'path'
import { config } from 'dotenv'

function getEnvPath() {
  if (process.env.ENV_FILE && fs.existsSync(resolve(process.env.ENV_FILE))) {
    return resolve(process.env.ENV_FILE)
  }

  if (fs.existsSync(resolve('../.env'))) {
    return resolve('../.env')
  }

  return resolve('../.env.e2e')
}

function loadDotenv(): { [key: string]: string } {
  const path = getEnvPath()

  const data =
    config({
      path
    }).parsed || {}

  if (!data.APP_CLIENT_URL) {
    data.APP_CLIENT_URL = data.APP_URL
  }

  return data
}

export default defineConfig({
  env: loadDotenv(),
  e2e: {
    setupNodeEvents(on, config) {
      return config
      // implement node event listeners here
    }
  }
})
