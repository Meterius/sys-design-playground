/** Actions that can appear in Valhalla status available_actions. */
export enum ValhallaAction {
  Route = 'route',
  OptimizedRoute = 'optimized_route',
  Matrix = 'sources_to_targets',
  Isochrone = 'isochrone',
  TraceRoute = 'trace_route',
  TraceAttributes = 'trace_attributes',
  Locate = 'locate',
  Height = 'height',
  Expansion = 'expansion',
  Status = 'status',
  Tile = 'tile',
  Centroid = 'centroid',
}

/** Status response.
 *
 * @see https://valhalla.github.io/valhalla/api/status/api-reference/
 */
export interface StatusResponse {
  /** Valhalla service version. */
  version?: string
  /** Unix timestamp for the routing tileset's last modification time. */
  tileset_last_modified?: number
  /** Actions advertised by the server. */
  available_actions?: ValhallaAction[]
  /** True when routing tiles are available. */
  has_tiles?: boolean
  /** Bounding box covered by the dataset. */
  bbox?: [number, number, number, number]
  /** Additional response values returned by Valhalla. */
  [key: string]: unknown
}
