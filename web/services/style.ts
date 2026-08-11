import { defineStore } from 'pinia'
import { basic } from '@/styles'
import { lightModeKey } from '@/config'

/**
 * The shell's chrome classes came from a two-variant theme picker inherited
 * with the dashboard template. Only `basic` was ever reachable — nothing wrote
 * the stored key — so the variants collapse into the one set that ships, and
 * theming is what `darkMode` does.
 */
export const store = defineStore('style', {
  state: () => ({
    asideStyle: basic.aside,
    asideBrandStyle: basic.asideBrand,
    asideMenuItemStyle: basic.asideMenuItem,
    asideMenuItemActiveStyle: basic.asideMenuItemActive,
    asideMenuDropdownStyle: basic.asideMenuDropdown,
    navBarItemLabelStyle: basic.navBarItemLabel,
    navBarItemLabelHoverStyle: basic.navBarItemLabelHover,
    navBarItemLabelActiveColorStyle: basic.navBarItemLabelActiveColor,
    overlayStyle: basic.overlay,

    darkMode: true
  }),
  actions: {
    setDarkMode(payload?: boolean) {
      this.darkMode = payload ?? !this.darkMode

      if (typeof localStorage !== 'undefined') {
        localStorage.setItem(lightModeKey, this.darkMode ? '0' : '1')
      }

      if (typeof document !== 'undefined') {
        // Layouts scope their own `dark` class, but the root scrollbar can
        // only be themed from <html> — stamp it there as well.
        document.documentElement.classList[this.darkMode ? 'add' : 'remove']('dark')
      }
    }
  }
})
