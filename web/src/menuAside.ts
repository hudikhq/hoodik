import { mdiMonitor, mdiLink, mdiViewDashboard, mdiHuman } from '@mdi/js'
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
    icon: mdiViewDashboard,
    label: 'Stats',
    roles: ['admin']
  },
  {
    to: { name: 'admin-users' },
    icon: mdiHuman,
    label: 'Users',
    roles: ['admin']
  }
] as AsideMenuItemType[]
