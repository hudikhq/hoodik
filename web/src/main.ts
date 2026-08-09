import { createApp } from 'vue'
import App from './App.vue'
import router from './router'
import { createPinia } from '!/init'
import { store as style } from '!/style'
import { lightModeKey, styleKey } from '@/config'
import { greeting } from '!/logger'
import { humanizeError } from '!/index'
import { i18n, currentLocale } from '@/i18n'
import Notifications, { notify } from '@kyvg/vue3-notification'
import './css/main.css'

greeting()

// Last resort for anything no handler caught. The reason's own message is
// the rawest string in the app, so it goes in the body under a title a
// person can read — never as the headline.
window.addEventListener('unhandledrejection', function (event) {
  notify({
    title: i18n.global.t('errors.requestFailed'),
    text: humanizeError(event.reason),
    type: 'error'
  })
})

const pinia = createPinia()

document.documentElement.setAttribute('lang', currentLocale())

/* Create Vue app */
createApp(App).use(i18n).use(Notifications).use(router).use(pinia).mount('#app')

/* Init Pinia stores */
const styleStore = style(pinia)

/* App style */
styleStore.setStyle(localStorage[styleKey] ?? 'basic')

/* Theme: dark unless the user explicitly stored a light preference */
styleStore.setDarkMode(localStorage[lightModeKey] !== '1')

/* Default title tag */
window.defaultDocumentTitle = import.meta.env.APP_NAME || 'Hoodik'

/* Set document title from route meta */
router.afterEach((to) => {
  document.title = to.meta?.title
    ? `${i18n.global.t(to.meta.title as string)} — ${window.defaultDocumentTitle}`
    : window.defaultDocumentTitle
})
