import { createApp } from 'vue'
import { createPinia } from 'pinia'

import App from './App.vue'
import router from './router'
import { store as style } from '@/stores/style'
import { lightModeKey, styleKey } from '@/config'

import './css/main.css'

/* Init Pinia */
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
