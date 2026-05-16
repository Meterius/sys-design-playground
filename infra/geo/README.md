# Geo Infrastructure

Docker Compose stack for the local map data services used by `jlh_maps`.
Run commands in this directory unless noted otherwise.

## Services

### `compose.yaml`

| Service | Purpose | Local endpoint | Data/setup dependency                                                                                                                                                                         |
| --- | --- | --- |-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `traefik` | Reverse proxy for the HTTP services in this stack. | `http://localhost:80`; dashboard on `http://localhost:8081` |                                                                                                                                                                                               |
| `postgres_osm` | PostGIS PostgreSQL database. Stores OSM data imported by the `osm2pgsql_osm_import` job. | PostgreSQL on `localhost:5433` | Automatically initialized from `postgres_osm/init/init.sql`; persisted in the `postgres_osm_data` Docker volume.                                                                              |
| `omt_tileserver_gl` | Serves OpenMapTiles vector tiles and styles through TileServer GL. | `http://tiles.jlh_maps.localhost` | Requires `${OPENMAPTILES_DIR}/data`, `${OPENMAPTILES_DIR}/style`, and `${OPENMAPTILES_DIR}/build`; Populated by output of https://github.com/Meterius/jlh-sys-design-playground-openmaptiles. |
| `raster_tile_json_server` | Static nginx server for Sentinel-2 raster TileJSON and XYZ PNG tiles. | `http://raster.jlh_maps.localhost/raster/sen2/tilejson.json` | Requires `${SAT_RASTER_TILE_JSON_DIR}` to contain `tilejson.json` plus the generated `{z}/{x}/{y}.png` tile tree. See `crates/sat_ingest` for populating raster tiles from satellite imagery. |
| `core_service` | Rust API for looking up imported OSM element metadata from `postgres_osm`. | `http://api.jlh_maps.localhost` | Requires the `unitable` table produced by the OSM import job of `postgres_osm`.                                                                                                               |
| `valhalla` | Valhalla routing service backed by a prebuilt routing graph. | `http://valhalla.jlh_maps.localhost` | Requires generated Valhalla files under `valhalla/custom_files`.                                                                                                                              |

### `compose.jobs.yaml`

| Service/job | Purpose | Inputs                                                                     | Output                                                                                       |
| --- | --- |----------------------------------------------------------------------------|----------------------------------------------------------------------------------------------|
| `osm2pgsql_osm_import` | One-shot OSM import job. It runs `osm2pgsql` in flex mode and loads OSM data into `postgres_osm`. | `postgres_osm/osm2pgsql/style.lua`; `https://download.geofabrik.de/europe` | Populates the `unitable` table for `postgres_osm`. |

Run the import job with the main stack file included so the `postgres_osm`
dependency is available:

```powershell
docker compose -f compose.yaml -f compose.jobs.yaml run --rm osm2pgsql_osm_import
```

## External Data And Setup

### Environment

Review `.env` before starting the stack:

```dotenv
POSTGRES_OSM_USER=...
POSTGRES_OSM_PASSWORD=...
POSTGRES_OSM_DB=...

OPENMAPTILES_DIR=...
SAT_RASTER_TILE_JSON_DIR=...
```

`OPENMAPTILES_DIR` and `SAT_RASTER_TILE_JSON_DIR` point to data prepared
outside this repository. They must be paths that Docker Desktop can mount.

### OpenMapTiles Vector Tiles

`omt_tileserver_gl` does not build vector tiles. Prepare an OpenMapTiles output
directory externally, then point `OPENMAPTILES_DIR` at it.

Required layout:

```text
${OPENMAPTILES_DIR}/
  data/
  style/
    config.json
  build/
```

`style/config.json` must reference the MBTiles and style assets available in
the mounted `data`, `style`, and `build` directories.

### Sentinel-2 Raster TileJSON

`raster_tile_json_server` only serves files. Generate the raster tiles before
starting the service. See `crates/sat_ingest` for an example of converting satellite imagery to the expected raster tile format.

### OSM Data For PostGIS

The `osm2pgsql_osm_import` job downloads data from Geofabrik and
imports it into `postgres_osm`. Edit the URL in `compose.jobs.yaml` if a
different extract is needed.

The job uses `postgres_osm/osm2pgsql/style.lua`, which writes all OSM object
types into one `unitable` table with `attrs`, `tags`, and `geom` columns.
`core_service` depends on that table.

### Valhalla Routing Data

`valhalla/custom_files` is ignored by git and must be prepared externally. The
running service expects the files referenced by `valhalla/custom_files/valhalla.json`,
including:

- `berlin.osm.pbf`
- `valhalla.json`
- `valhalla_tiles/` or `valhalla_tiles.tar`
- `admins.sqlite`
- `timezones.sqlite`
- `default_speeds.json`

Generate these artifacts with Valhalla tooling for the same OSM extract you
want to route over, then place them under `infra/geo/valhalla/custom_files`
before starting the `valhalla` service.

## Running The Stack

After external data is in place:

```powershell
docker compose up -d --build
```
