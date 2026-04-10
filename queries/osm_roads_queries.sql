--! list_all_roads
SELECT
    osm_id, class, category, oneway, max_speed, layer,
    is_bridge, is_tunnel, ST_asewkb(geom::geometry) as geom
FROM osm_roads;

--! upsert_road
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
    osm_id, class, category, oneway, max_speed, layer, is_bridge, is_tunnel, geom
)
SELECT
    s.osm_id,
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
    (class, category, oneway, max_speed, layer, is_bridge, is_tunnel, geom) =
        (excluded.class, excluded.category, excluded.oneway, excluded.max_speed, excluded.layer, excluded.is_bridge, excluded.is_tunnel, excluded.geom);

--! upsert_roads_streaming_end
DROP TABLE tmp_upsert_roads_streaming;