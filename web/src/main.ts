import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import { createPinia } from '!/init'
import { store as style } from '!/style'
import { lightModeKey, styleKey } from '@/config'
import { greeting } from '!/logger'
import * as logger from '!/logger'
import Notifications, { notify } from '@kyvg/vue3-notification'
import './css/main.css'

greeting()

// @ts-ignore
import { serviceWorkerFile } from 'virtual:vite-plugin-service-worker'

try {
  if ('Worker' in window) {
    logger.debug('[main] Creating UPLOAD worker from', serviceWorkerFile)
    window.UPLOAD = new Worker(serviceWorkerFile, { type: 'module', name: 'Hoodik Upload Worker' })

    logger.debug('[main] Creating DOWNLOAD worker from', serviceWorkerFile)
    window.DOWNLOAD = new Worker(serviceWorkerFile, {
      type: 'module',
      name: 'Hoodik Download Worker'
    })

    logger.debug('[main] Creating HASH worker')
    window.HASH = new Worker(new URL('../hash-worker.ts', import.meta.url), {
      type: 'module',
      name: 'Hoodik Hash Worker'
    })

    logger.debug('[main] Workers created successfully')
  } else {
    logger.warn('[main] Worker API not available — uploads/downloads will run on main thread')
  }
} catch (error) {
  logger.error('[main] Worker creation failed:', error)
}

window.addEventListener('unhandledrejection', function (event) {
  notify({
    title: event.reason.message || 'Something went wrong',
    text: event.reason.description,
    type: 'error'
  })
})

const pinia = createPinia()

/* Create Vue app */
createApp(App).use(Notifications).use(router).use(pinia).mount('#app')

/* Init Pinia stores */
const styleStore = style(pinia)

/* App style */
styleStore.setStyle(localStorage[styleKey] ?? 'basic')

/* Dark mode */
if (
  (!localStorage[lightModeKey] && window.matchMedia('(prefers-color-scheme: dark)').matches) ||
  localStorage[lightModeKey] === '1'
) {
  styleStore.setDarkMode(true)
}

/* Default title tag */
window.defaultDocumentTitle = import.meta.env.APP_NAME || 'Hoodik'

/* Set document title from route meta */
router.afterEach((to) => {
  document.title = to.meta?.title
    ? `${to.meta.title} — ${window.defaultDocumentTitle}`
    : window.defaultDocumentTitle
})
