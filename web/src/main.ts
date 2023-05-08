import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import { createPinia } from '!/init'
import { store as style } from '!/style'
import { lightModeKey, styleKey } from '@/config'
import { greeting } from '!/logger'
import * as logger from '!/logger'

greeting()
// @ts-ignore
import { serviceWorkerFile } from 'virtual:vite-plugin-service-worker'

try {
  if ('Worker' in window) {
    window.UPLOAD = new Worker(serviceWorkerFile, { type: 'module', name: 'Hoodik Upload Worker' })
    window.DOWNLOAD = new Worker(serviceWorkerFile, {
      type: 'module',
      name: 'Hoodik Download Worker'
    })
  }
} catch (error) {
  logger.error('Registration failed', error)
}

import './css/main.css'

const pinia = createPinia()

/* Create Vue app */
createApp(App).use(router).use(pinia).mount('#app')

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
const defaultDocumentTitle = 'Hoodik - End 2 End Encrypted File Storage'

/* Set document title from route meta */
router.afterEach((to) => {
  document.title = to.meta?.title
    ? `${to.meta.title} — ${defaultDocumentTitle}`
    : defaultDocumentTitle
})
