-- Enable PostGIS extension
CREATE EXTENSION IF NOT EXISTS postgis;

-- Create locations table
CREATE TABLE locations (
    id SERIAL PRIMARY KEY,
    tags TEXT NOT NULL,
    latitude DOUBLE PRECISION NOT NULL,
    longitude DOUBLE PRECISION NOT NULL,
    geom GEOGRAPHY(Point, 4326), -- PostGIS spatial column
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Optional: populate geom from lat/lon
CREATE OR REPLACE FUNCTION update_geom()
RETURNS TRIGGER AS $$
BEGIN
    NEW.geom = ST_SetSRID(ST_MakePoint(NEW.longitude, NEW.latitude), 4326);
RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_geom
    BEFORE INSERT OR UPDATE ON locations
                         FOR EACH ROW
                         EXECUTE FUNCTION update_geom();

-- Spatial index for performance
CREATE INDEX idx_locations_geom ON locations USING GIST (geom);