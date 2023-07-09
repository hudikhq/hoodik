import { mdiMonitor, mdiLink, mdiHuman, mdiHumanMaleFemale, mdiCog } from '@mdi/js'
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
    label: 'My links'
  },
  {
    to: { name: 'account' },
    icon: mdiHuman,
    label: 'My Account'
  },
  {
    to: { name: 'admin-dashboard' },
    icon: mdiHumanMaleFemale,
    label: 'App Users',
    roles: ['admin']
  },
  {
    to: { name: 'admin-settings' },
    icon: mdiCog,
    label: 'App Settings',
    roles: ['admin']
  }
] as AsideMenuItemType[]
