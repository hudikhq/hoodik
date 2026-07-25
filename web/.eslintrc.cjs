/* eslint-env node */
require('@rushstack/eslint-patch/modern-module-resolution')

module.exports = {
  root: true,
  'extends': [
    'plugin:vue/vue3-essential',
    'eslint:recommended',
    '@vue/eslint-config-typescript',
    '@vue/eslint-config-prettier/skip-formatting'
  ],
  parserOptions: {
    ecmaVersion: 'latest'
  },
  overrides: [
    {
      files: ['src/**/*.vue'],
      extends: ['plugin:@intlify/vue-i18n/base'],
      rules: {
        // Keep hardcoded UI copy out of templates now that everything runs
        // through the locale catalogs. Symbols, numbers and punctuation-only
        // text are fine.
        '@intlify/vue-i18n/no-raw-text': [
          'error',
          {
            ignorePattern: '^[-–—#:()\\[\\]&+;.,·…*/\\s\\d%$€?°×↓↑|]+$'
          }
        ]
      }
    }
  ],
  settings: {
    'vue-i18n': {
      localeDir: './src/locales/*.json',
      messageSyntaxVersion: '^9.0.0'
    }
  },
  ignorePatterns: [
    'dist/',
    'node_modules/',
    'playwright-report/',
    'test-results/',
    'coverage/'
  ]
}
