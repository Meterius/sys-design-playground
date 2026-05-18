# jlh_maps

Vue 3 + Vite frontend for the JLH maps application. It renders the MapLibre map UI, routing controls, custom layers, and the Rust WASM map integration from `crates/jlh_maps_app`.

## Commands

```sh
npm install
npm run dev
npm run build
npm run test:unit
npm run test:e2e
npm run lint
```

## Structure

```text
├── e2e/       Playwright end-to-end tests.
├── public/    Static assets served by Vite.
└── src/       Vue application source.
```

## Source Layout

```text
src/
├── assets/            App styles and bundled assets.
├── components/        Reusable Vue UI components.
├── composables/       Shared Vue composition functions.
├── external/          Boundaries for external libraries and generated clients.
├── maplibre-layers/   Custom MapLibre layers and overlays.
├── router/            Vue Router setup.
├── runtime/           Runtime configuration and app wiring.
├── shaders/           GLSL shader sources.
├── stores/            Pinia stores.
├── types/             Shared TypeScript types.
├── utils/             General frontend utilities.
├── views/             Route-level Vue views.
└── wasm/              WASM integration code.
```
