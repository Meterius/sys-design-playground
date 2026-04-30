import {type OsmId, OsmType} from "@/external/osm.ts";

export const TILESERVER_URL = new URL(import.meta.env.VITE_TILESERVER_OMT_URL)
export const API_URL = new URL(import.meta.env.VITE_API_URL)

export const TILESERVER_OMT_DEFAULT_STYLE_TILEJSON_URL = new URL(
  'styles/omt_default/style.json',
  TILESERVER_URL,
)

export interface OsmData {
    tags: Record<string, string>,
    attrs: Record<string, string | number>,
}

export async function getOsmData(osm_id: OsmId): Promise<OsmData | null> {
    const type = {
        [OsmType.Node]: 'node',
        [OsmType.Way]: 'way',
        [OsmType.Relation]: 'relation',
    }[osm_id.type];

    const res = await fetch(new URL(`/osm/element/${type}/${osm_id.key}`, API_URL));

    if (res.ok) {
        return res.json();
    } else if (res.status === 404) {
        return null;
    }

    throw new Error(`Failed to fetch OSM data for ${osm_id.type}/${osm_id.key}: ${res.status} ${res.statusText}`);
}
