import type { LatLon } from './common.js'

/** Elevation lookup request.
 *
 * @see https://valhalla.github.io/valhalla/api/elevation/api-reference/
 */
export interface HeightRequest {
  /** Locations for which elevation should be sampled. */
  shape: LatLon[]
  /** Return cumulative range-height pairs when true. */
  range?: boolean
  /** Encoded polyline alternative to shape. */
  encoded_polyline?: string
  /** Correlation id echoed by Valhalla. */
  id?: string
  /** Valhalla accepts endpoint-specific extension fields. */
  [key: string]: unknown
}

/** One elevation sample. */
export interface HeightPoint extends LatLon {
  /** Height in meters, or null when no elevation is available. */
  height: number | null
}

/** Elevation lookup response. */
export interface HeightResponse {
  /** Shape points with heights. */
  shape?: HeightPoint[]
  /** Heights for requested points. */
  height?: Array<number | null>
  /** Cumulative distance and height pairs when range is requested. */
  range_height?: Array<[number, number | null]>
  /** Correlation id echoed from the request. */
  id?: string
  /** Additional response values returned by Valhalla. */
  [key: string]: unknown
}
