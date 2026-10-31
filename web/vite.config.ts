import path from 'node:path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

const webUi = path.resolve(__dirname, '../../agentzoo/zoo-web-ui/src')

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: [
      { find: '@agentzoo/web-ui', replacement: webUi },
    ],
  },
  base: '/',
  server: {
    port: 5175,
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
    outDir: '../../agentzoo/zoo-server/web/dist',
    emptyOutDir: true,
  },
})
