import path from 'node:path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

const webUi = path.resolve(__dirname, 'node_modules/@agentzoo/web-ui/zoo-web-ui/src')

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: [
      { find: /^@agentzoo\/web-ui$/, replacement: path.join(webUi, 'index.ts') },
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
    outDir: './dist',
    emptyOutDir: true,
  },
})
