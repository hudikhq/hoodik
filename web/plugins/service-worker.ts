import type { PluginOption } from 'vite'

type Options = {
  filename: string
}

export function serviceWorkerPlugin(options: Options): PluginOption {
  const name = 'vite-plugin-service-worker'
  const virtualModuleId = `virtual:${name}`
  const resolvedVirtualModuleId = '\0' + virtualModuleId
  let isBuild = false
  return {
    name,
    config(_, { command }) {
      isBuild = command === 'build'
      return {
        build: {
          rollupOptions: {
            input: {
              main: 'index.html',
              sw: options.filename
            },
            output: {
              entryFileNames: ({ facadeModuleId }) => {
                if (facadeModuleId?.includes(options.filename)) {
                  return `[name].js`
                }
                return 'assets/[name].[hash].js'
              }
            }
          }
        }
      }
    },
    resolveId(id) {
      if (id === virtualModuleId) {
        return resolvedVirtualModuleId
      }
    },
    load(id) {
      if (id === resolvedVirtualModuleId) {
        let filename = isBuild ? options.filename.replace('.ts', '.js') : options.filename
        if (!filename.startsWith('/')) filename = `/${filename}`
        return `export const serviceWorkerFile = '${filename}'`
      }
    }
  }
}
