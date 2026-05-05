import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueJsx from '@vitejs/plugin-vue-jsx'
import ui from '@nuxt/ui/vite'
import vueDevTools from 'vite-plugin-vue-devtools'
import wasm from "vite-plugin-wasm"

// https://vite.dev/config/
export default defineConfig({
  server: {
    fs: {
      allow: ['./', '../../crates/jlh_maps_frontend/pkg', '../../crates/jlh_maps_app/pkg'],
    },
  },
  plugins: [wasm(), vue(), ui(), vueJsx(), vueDevTools()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
})
