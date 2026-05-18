# jlh-maps

Simple map application.

## Structure

```text
├── crates/       Rust workspace crates.
├── infra/        Docker Compose and service infrastructure. See infra/README.md.
└── packages/     Frontend and TypeScript packages.
```

## Crates

- `jlh_maps_app` - Rust/Bevy code built for the map frontend through WASM.
- `jlh_maps_core_service` - Actix/Juniper backend service for map data APIs.
- `sat_ingest` - Sentinel/satellite ingestion and conversion tooling.
- `utilities` - Shared Rust geometry, image, and data utilities.

## Packages

- `jlh_maps` - Vue/Vite map frontend.
- `valhalla_client` - Typed TypeScript client for Valhalla API endpoints.

## Datasets

- OSM extracts: https://download.geofabrik.de, usually as `.osm.pbf`.
