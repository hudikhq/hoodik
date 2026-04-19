import { defineConfig } from 'vite'
import path from 'path'
import { viteSingleFile } from 'vite-plugin-singlefile'

export default defineConfig(({ mode }) => {
  if (mode === 'standalone') {
    return {
      root: path.resolve(__dirname, 'src/standalone'),
      plugins: [viteSingleFile()],
      build: {
        outDir: path.resolve(__dirname, 'dist/standalone'),
        emptyOutDir: true,
      },
    }
  }

  // Library mode (default)
  return {
    build: {
      lib: {
        entry: path.resolve(__dirname, 'src/index.ts'),
        formats: ['es'],
        fileName: 'index',
      },
      outDir: path.resolve(__dirname, 'dist'),
      emptyOutDir: false,
      rollupOptions: {
        external: [
          /^@milkdown\//,
          /^refractor/,
          'dompurify',
        ],
      },
    },
  }
})
