import type { Geometry } from 'geojson'
import type { BaseRoutingRequest, RouteResponse, ShapeFormat, ValhallaLocation } from './common.js'

/** Turn-by-turn route request.
 *
 * @see https://valhalla.github.io/valhalla/api/turn-by-turn/api-reference/
 */
export interface RouteRequest extends BaseRoutingRequest {
  /** Number of alternative routes to request. */
  alternates?: number
  /** Locations near which edges should be excluded from the route. */
  exclude_locations?: ValhallaLocation[]
  /** GeoJSON polygons whose intersecting edges should be excluded. */
  exclude_polygons?: Geometry[]
  /** Shape encoding requested in the response. */
  shape_format?: ShapeFormat
}

export type { RouteResponse }
