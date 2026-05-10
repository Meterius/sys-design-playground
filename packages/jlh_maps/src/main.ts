import './assets/main.css'
import './runtime/virtual-webgl2'

import initFr from 'jlh_maps_frontend'
import initApp, { initialize } from 'jlh_maps_app'

import { createApp } from 'vue'
import { createPinia } from 'pinia'
import VueMaplibreGl from '@indoorequal/vue-maplibre-gl'
import ui from '@nuxt/ui/vue-plugin'

import App from './App.vue'
import router from './router'

Promise.all([initFr(), initApp()])
  .catch((err) => {
    console.error('WASM Initialization Failure: ', err)
  })
  .finally(() => {
    initialize()

    const app = createApp(App)

    app.use(VueMaplibreGl)
    app.use(createPinia())
    app.use(router)
    app.use(ui)

    app.mount('#app')
  })
