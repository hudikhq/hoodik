import { createRouter, createWebHistory } from 'vue-router'
import FilesView from '../views/FilesView.vue'

const router = createRouter({
  history: createWebHistory(`/`),
  routes: [
    {
      path: '/:file_id?',
      name: 'Dashboard',
      meta: {
        files: true
      },
      component: FilesView
    },
    {
      path: '/auth/setup-lock-screen',
      name: 'Setup Lock Screen',
      component: () => import('../views/auth/SetupLockScreenView.vue')
    },
    {
      path: '/auth/decrypt',
      name: 'Decrypt Private Key',
      component: () => import('../views/auth/DecryptView.vue')
    },
    {
      path: '/auth/lock',
      name: 'Account Locked',
      component: () => import('../views/auth/LockView.vue')
    },
    {
      path: '/auth/login/private-key',
      name: 'Login',
      component: () => import('../views/auth/login/PrivateKeyView.vue')
    },
    {
      path: '/auth/login',
      name: 'Login - Credentials',
      component: () => import('../views/auth/login/IndexView.vue')
    },
    {
      path: '/auth/register',
      name: 'Create Account - Credentials',
      component: () => import('../views/auth/register/IndexView.vue')
    },
    {
      path: '/auth/register/key',
      name: 'Create Account - Your Private Key',
      component: () => import('../views/auth/register/KeyView.vue')
    },
    {
      path: '/auth/register/two-factor',
      name: 'Create Account - Two Factor Authentication',
      component: () => import('../views/auth/register/TwoFactorView.vue')
    }
  ]
})

export default router
