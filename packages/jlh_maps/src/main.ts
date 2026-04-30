import './assets/main.css'

import { createApp } from 'vue'
import { createPinia } from 'pinia'
import VueMaplibreGl from '@indoorequal/vue-maplibre-gl'
import ui from '@nuxt/ui/vue-plugin'

import App from './App.vue'
import router from './router'

const app = createApp(App)

app.use(VueMaplibreGl)
app.use(createPinia())
app.use(router)
app.use(ui)

app.mount('#app')
