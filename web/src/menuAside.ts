import { mdiMonitor, mdiLink, mdiHuman, mdiCog } from '@mdi/js'
import type { RouteLocation } from 'vue-router'

export interface AsideMenuItemType {
  to: RouteLocation
  icon: string
  label: string
  roles?: string[]
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
  },
  {
    to: { name: 'admin-dashboard' },
    icon: mdiHuman,
    label: 'Manage',
    roles: ['admin']
  },
  {
    to: { name: 'admin-settings' },
    icon: mdiCog,
    label: 'Settings',
    roles: ['admin']
  }
] as AsideMenuItemType[]
