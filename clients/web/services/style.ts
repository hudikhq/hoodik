import { defineStore } from 'pinia'
import * as styles from '@/styles'
import { lightModeKey, styleKey } from '@/config'

export const store = defineStore('style', {
  state: () => ({
    /* Styles */
    asideStyle: 'basic',
    asideScrollbarsStyle: 'basic',
    asideBrandStyle: 'basic',
    asideMenuItemStyle: 'basic',
    asideMenuItemActiveStyle: 'basic',
    asideMenuDropdownStyle: 'basic',
    navBarItemLabelStyle: 'basic',
    navBarItemLabelHoverStyle: 'basic',
    navBarItemLabelActiveColorStyle: 'basic',
    overlayStyle: 'basic',

    /* Dark mode */
    darkMode: true
  }),
  actions: {
    setStyle(payload: 'white' | 'basic') {
      if (!styles[payload]) {
        return
      }

      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(styleKey, payload)
      }

      const style = styles[payload]

      for (const key in style) {
        // @ts-ignore
        this[`${key}Style`] = style[key]
      }
    },

    setDarkMode(payload?: boolean) {
      this.darkMode = payload ? payload : !this.darkMode

      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(lightModeKey, this.darkMode ? '0' : '1')
      }

      if (typeof document !== 'undefined') {
        document.body.classList[this.darkMode ? 'add' : 'remove']('dark-scrollbars')

        document.documentElement.classList[this.darkMode ? 'add' : 'remove'](
          'dark-scrollbars-compat'
        )
      }
    }
  }
})
