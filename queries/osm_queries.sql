--! list_all_roads
SELECT
    osm_id, class, category, oneway, max_speed, layer, reference,
    is_bridge, is_tunnel, ST_asewkb(geom::geometry) as geom
FROM osm_roads;

--! fetch_roads_by_area : (max_speed?)
SELECT
    osm_id, class, category, oneway, max_speed, layer, reference,
    is_bridge, is_tunnel, ST_asewkb(geom::geometry) as geom
FROM osm_roads
WHERE st_intersects(geom, st_setsrid(st_geomfromewkb(:bounds), 4326)::geography);

--! fetch_roads_by_area_and_category : (max_speed?)
SELECT
    osm_id, class, category, oneway, max_speed, layer, reference,
    is_bridge, is_tunnel, ST_asewkb(geom::geometry) as geom
FROM osm_roads
WHERE category = :category AND st_intersects(geom, st_setsrid(st_geomfromewkb(:bounds), 4326)::geography);

--! upsert_road (max_speed?)
INSERT INTO osm_roads (
    osm_id, class, category, oneway, max_speed, layer,
    is_bridge, is_tunnel, geom
)
VALUES
    (:osm_id, :class, :category, :oneway, :max_speed, :layer, :is_bridge, :is_tunnel, st_setsrid(st_geomfromewkb(:geom), 4326)::geography)
ON CONFLICT(osm_id)
DO UPDATE SET
    (class, category, oneway, max_speed, layer, is_bridge, is_tunnel, geom) =
    (excluded.class, excluded.category, excluded.oneway, excluded.max_speed, excluded.layer, excluded.is_bridge, excluded.is_tunnel, excluded.geom);

--! upsert_roads_streaming_start
CREATE TEMP TABLE tmp_upsert_roads_streaming AS SELECT * FROM osm_roads LIMIT 0;

--! upsert_road_streaming_transfer
COPY tmp_upsert_roads_streaming (
    osm_id, class, category, oneway, max_speed, layer,
    is_bridge, is_tunnel, geom
) FROM stdin binary;

--! upsert_roads_streaming_commit
INSERT INTO osm_roads (
    osm_id, reference, class, category, oneway, max_speed, layer, is_bridge, is_tunnel, geom
)
SELECT
    s.osm_id,
    s.reference,
    s.class,
    s.category,
    s.oneway,
    s.max_speed,
    s.layer,
    s.is_bridge,
    s.is_tunnel,
    st_setsrid(st_geomfromewkb(s.geom), 4326)::geography
FROM tmp_upsert_roads_streaming s
ON CONFLICT(osm_id)
    DO UPDATE SET
    (reference, class, category, oneway, max_speed, layer, is_bridge, is_tunnel, geom) =
        (excluded.reference, excluded.class, excluded.category, excluded.oneway, excluded.max_speed, excluded.layer, excluded.is_bridge, excluded.is_tunnel, excluded.geom);

--! upsert_roads_streaming_end
DROP TABLE tmp_upsert_roads_streaming;

--! upsert_buildings_streaming_start
CREATE TEMP TABLE tmp_upsert_buildings_streaming AS SELECT * FROM osm_buildings LIMIT 0;

--! upsert_buildings_streaming_transfer
COPY tmp_upsert_buildings_streaming (
    osm_id, kind, geom
) FROM stdin binary;

--! upsert_buildings_streaming_commit
INSERT INTO osm_buildings (
    osm_id, kind, geom
)
SELECT
    s.osm_id,
    s.kind,
    st_setsrid(st_geomfromewkb(s.geom), 4326)::geography
FROM tmp_upsert_buildings_streaming s
ON CONFLICT(osm_id)
    DO UPDATE SET
    (kind, geom) = (excluded.kind, excluded.geom);

--! upsert_buildings_streaming_end
DROP TABLE tmp_upsert_buildings_streaming;

--! fetch_buildings_by_area : (kind?)
SELECT
    osm_id, kind, ST_asewkb(geom::geometry) as geom
FROM osm_buildings
WHERE st_intersects(geom, st_setsrid(st_geomfromewkb(:bounds), 4326)::geography);

--! upsert_waters_streaming_start
CREATE TEMP TABLE tmp_upsert_waters_streaming AS SELECT * FROM osm_waters LIMIT 0;

--! upsert_waters_streaming_transfer
COPY tmp_upsert_waters_streaming (
                                     osm_id, class, geom
    ) FROM stdin binary;

--! upsert_waters_streaming_commit
INSERT INTO osm_waters (
    osm_id, class, geom
)
SELECT
    s.osm_id,
    s.class,
    st_setsrid(st_geomfromewkb(s.geom), 4326)::geography
FROM tmp_upsert_waters_streaming s
ON CONFLICT(osm_id)
    DO UPDATE SET
    (class, geom) = (excluded.class, excluded.geom);

--! upsert_waters_streaming_end
DROP TABLE tmp_upsert_waters_streaming;

--! fetch_waters_by_area
SELECT
    osm_id, class, ST_asewkb(geom::geometry) as geom
FROM osm_waters
WHERE st_intersects(geom, st_setsrid(st_geomfromewkb(:bounds), 4326)::geography);

--! upsert_landuses_streaming_start
CREATE TEMP TABLE tmp_upsert_landuses_streaming AS SELECT * FROM osm_landuses LIMIT 0;

--! upsert_landuses_streaming_transfer
COPY tmp_upsert_landuses_streaming (
                                  osm_id, class, geom
    ) FROM stdin binary;

--! upsert_landuses_streaming_commit
INSERT INTO osm_landuses (
    osm_id, class, geom
)
SELECT
    s.osm_id,
    s.class,
    st_setsrid(st_geomfromewkb(s.geom), 4326)::geography
FROM tmp_upsert_landuses_streaming s
ON CONFLICT(osm_id)
    DO UPDATE SET
    (class, geom) = (excluded.class, excluded.geom);

--! upsert_landuses_streaming_end
DROP TABLE tmp_upsert_landuses_streaming;

--! fetch_landuses_by_area
SELECT
    osm_id, class, ST_asewkb(geom::geometry) as geom
FROM osm_landuses
WHERE st_intersects(geom, st_setsrid(st_geomfromewkb(:bounds), 4326)::geography);
