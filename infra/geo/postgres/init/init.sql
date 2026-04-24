CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TYPE WATER_CLASS AS ENUM (
    'water','reservoir','river','dock','glacier','wetland','wetland_fen','riverbank','wetland_mangrove','wetland_marsh',
    'wetland_tidalflat', 'wetland_reedbed', 'wetland_wet_meadow', 'wetland_swamp', 'wetland_saltmarsh', 'wetland_bog'
);

CREATE TYPE ROAD_CLASS AS ENUM (
    'motorway','trunk','primary','secondary','tertiary',
    'unclassified','residential','living_street','pedestrian','busway',
    'motorway_link','trunk_link','primary_link','secondary_link','tertiary_link',
    'service','track','track_grade1','track_grade2','track_grade3','track_grade4','track_grade5',
    'bridleway','cycleway','footway','path','steps','unknown'
);

CREATE TYPE ROAD_CLASS_CATEGORY AS ENUM (
    'major_roads', 'minor_roads', 'highway_links', 'very_small_roads',
    'paths_unsuitable_for_cars', 'unknown'
);

CREATE TYPE ROAD_ONEWAY AS ENUM ('forwards_only','backwards_only','bidirectional');

CREATE TABLE osm_roads (
  osm_id        bigint primary key,
  class         ROAD_CLASS not null,
  category      ROAD_CLASS_CATEGORY not null,
  reference     text not null,
  oneway        ROAD_ONEWAY not null,
  max_speed     integer,
  layer         integer not null,
  is_bridge     boolean not null,
  is_tunnel     boolean not null,
  geom          geography(LINESTRING, 4326) not null
);

CREATE INDEX idx_osm_roads_geom ON osm_roads USING GIST (geom);

CREATE TABLE tmp_upsert_roads_streaming AS SELECT * FROM osm_roads LIMIT 0;

CREATE TABLE osm_buildings (
    osm_id        bigint primary key,
    kind          text,
    geom          geography(MULTIPOLYGON, 4326) not null
);

CREATE INDEX idx_osm_buildings_geom ON osm_buildings USING GIST (geom);

CREATE TABLE tmp_upsert_buildings_streaming AS SELECT * FROM osm_buildings LIMIT 0;

CREATE TABLE osm_waters (
    osm_id        bigint primary key,
    class         WATER_CLASS not null,
    geom          geography(MULTIPOLYGON, 4326) not null
);

CREATE INDEX idx_osm_waters_geom ON osm_waters USING GIST (geom);

CREATE TABLE tmp_upsert_waters_streaming AS SELECT * FROM osm_waters LIMIT 0;
