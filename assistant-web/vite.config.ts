import path from 'node:path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { viteSingleFile } from 'vite-plugin-singlefile'

const webUi = path.resolve(__dirname, '../../agentzoo/zoo-web-ui/src')

export default defineConfig({
  plugins: [react(), viteSingleFile()],
  resolve: {
    alias: [
      { find: '@agentzoo/web-ui/global.css', replacement: path.join(webUi, 'global.css') },
      { find: /^@agentzoo\/web-ui$/, replacement: path.join(webUi, 'index.ts') },
      { find: '@agentzoo/web-ui', replacement: webUi },
    ],
  },
  server: {
    port: 5174,
    proxy: {
      '/projects': 'http://127.0.0.1:9211',
      '/models': 'http://127.0.0.1:9211',
      '/health': 'http://127.0.0.1:9211',
      '/events': 'http://127.0.0.1:9211',
      '/user': 'http://127.0.0.1:9211',
      '/agents': 'http://127.0.0.1:9211',
    },
  },
  build: {
    outDir: '../zoo-plugin-assistant/ui',
    emptyOutDir: true,
    assetsInlineLimit: 100000000,
    cssCodeSplit: false,
  },
})
