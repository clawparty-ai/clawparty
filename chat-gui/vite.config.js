import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
	publicDir: 'public',
  plugins: [vue()],
  clearScreen: false,
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:6789',
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, '/api')
      }
    },
		port: 1420,
		strictPort: true,
		host: host || false,
		hmr: host
		  ? {
		      protocol: "ws",
		      host,
		      port: 1421,
		    }
		  : undefined,
		watch: {
		  // 3. tell Vite to ignore watching `src-tauri`
		  ignored: ["**/src-tauri/**"],
		},
  }
})