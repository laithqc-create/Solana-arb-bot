// src-tauri/frontend/vite.config.ts
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  root: 'src',
  build: {
    target: 'ES2021',
    minify: 'terser',
    outDir: '../dist',
    emptyOutDir: true,
  },
  server: {
    port: 5173,
  }
})
