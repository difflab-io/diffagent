import { sveltekit } from '@sveltejs/kit/vite'
import tailwindcss from '@tailwindcss/vite'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],
  server: {
    host: '127.0.0.1',
    port: Number(process.env.DIFFAGENT_WEB_PORT ?? 1420),
    strictPort: Boolean(process.env.DIFFAGENT_WEB_PORT),
  },
})
