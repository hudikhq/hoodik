import { createPinia as _createPinia } from 'pinia'
import localForage from 'localforage'

export function createPinia() {
  localForage.config({
    name: 'hp-pinia-store',
    driver: localForage.INDEXEDDB
  })

  const pinia = _createPinia()
  // pinia.use(async ({ store }) => {
  // const stored = await localForage.getItem(store.$id)
  // if (stored) {
  //   store.$patch(stored)
  // }
  // store.$subscribe(() => {
  //   localForage.setItem(store.$id, { ...store.$state })
  // })
  // })

  return pinia
}
