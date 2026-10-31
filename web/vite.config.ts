import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
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
