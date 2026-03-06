import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
	publicDir: 'public',
  plugins: [vue()],
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:6789',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, '/api')
      }
    }
  }
})