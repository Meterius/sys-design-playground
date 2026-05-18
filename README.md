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

## Useful Links

| Title | Description | Website | Usage description | Infra Usage | Code Usage |
| --- | --- | --- | --- | --- | --- |
| A/B Street | Open source city traffic simulation and planning tool; useful as a reference for map-based urban interaction ideas. | https://a-b-street.github.io/ | None | None | None |
| OpenMapTiles | Vector tile schema and tooling for serving OSM-derived basemaps. | https://openmaptiles.org/ | Used to produce TileJSON and MapLibre styles which are queried/rendered via `maplibre_gl_js`. | `omt_tileserver_gl`; `traefik` | `packages/jlh_maps`; `crates/jlh_maps_app` |
| Valhalla | Open source routing engine used for route requests and route display. | https://github.com/valhalla/valhalla | Runs as the routing service; the frontend queries it through the typed `valhalla_client`. | `valhalla`; `traefik` | `packages/valhalla_client`; `packages/jlh_maps` |
| TileServer GL | Server for Mapbox GL styles, vector tiles, and rasterized map tiles. | https://github.com/maptiler/tileserver-gl | Serves OpenMapTiles styles and tiles consumed by the frontend map. | `omt_tileserver_gl`; `traefik` | `packages/jlh_maps` |
| nginx | HTTP server used here for static raster TileJSON and tile assets. | https://nginx.org/ | Serves generated Sentinel-2 raster TileJSON and XYZ PNG tiles. | `raster_tile_json_server`; `traefik` | `packages/jlh_maps` |
| Traefik | Reverse proxy that routes local hostnames to Compose services. | https://github.com/traefik/traefik | Routes local hostnames such as `tiles.jlh_maps.localhost` and `api.jlh_maps.localhost` to Docker Compose services. | `traefik`; `omt_tileserver_gl`; `raster_tile_json_server`; `core_service`; `valhalla` | `packages/jlh_maps` |
| PostgreSQL | Relational database used for imported OSM metadata. | https://www.postgresql.org/ | Stores imported OSM metadata queried by the Rust core service. | `postgres_osm`; `core_service`; `osm2pgsql_osm_import` | `crates/jlh_maps_core_service`; `packages/jlh_maps` |
| PostGIS | PostgreSQL extension for spatial geometry and GIS queries. | https://postgis.net/ | Adds spatial types and GIS capabilities to the OSM PostgreSQL database. | `postgres_osm`; `osm2pgsql_osm_import` | `crates/jlh_maps_core_service` |
| osm2pgsql | OSM import tool used to load `.osm.pbf` extracts into PostgreSQL/PostGIS. | https://osm2pgsql.org/ | Imports OSM extracts into the `unitable` schema consumed by `core_service`. | `osm2pgsql_osm_import`; `postgres_osm` | `crates/jlh_maps_core_service` |
