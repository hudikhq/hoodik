import { mdiMonitor, mdiLink } from '@mdi/js'
import type { RouteLocation } from 'vue-router'

export interface AsideMenuItemType {
  to: RouteLocation
  icon: string
  label: string
}

export default [
  {
    to: { name: 'files' },
    icon: mdiMonitor,
    label: 'My Files'
  },
  {
    to: { name: 'links' },
    icon: mdiLink,
    label: 'Public links'
  }
] as AsideMenuItemType[]
