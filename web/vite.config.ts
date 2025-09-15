import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  root: './',
  publicDir: './public',
  build: {
    outDir: './build',
    rollupOptions: {
      input: './index.html'
    }
  },
  server: {
    host: 'localhost',
    port: 3000,
    cors: true,
    headers: {
      'Cross-Origin-Embedder-Policy': 'require-corp',
      'Cross-Origin-Opener-Policy': 'same-origin',
    }
  },
  optimizeDeps: {
    exclude: ['@vite/client', '@vite/env']
  },
  worker: {
    format: 'es',
    plugins: []
  }
})
