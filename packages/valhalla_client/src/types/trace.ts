import type {
  CostingModel,
  CostingOptionsByModel,
  LatLon,
  RouteResponse,
} from './common.js'

/** Map matching algorithm preference.
 *
 * @see https://valhalla.github.io/valhalla/api/map-matching/api-reference/
 */
export enum TraceShapeMatch {
  EdgeWalk = 'edge_walk',
  MapSnap = 'map_snap',
  WalkOrSnap = 'walk_or_snap',
}

/** Whether trace attribute filters include or exclude listed attributes. */
export enum AttributeFilterAction {
  Include = 'include',
  Exclude = 'exclude',
}

/** One input trace point for map matching. */
export interface TracePoint extends LatLon {
  /** Optional timestamp for the point. */
  time?: string
  /** GPS accuracy in meters. */
  accuracy?: number
  /** Heading in clockwise degrees from north. */
  heading?: number
  /** Additional point metadata. */
  [key: string]: unknown
}

/** Shared map matching request fields. */
export interface TraceRequest {
  /** Input GPS trace. */
  shape: TracePoint[]
  /** Costing model used while matching. */
  costing: CostingModel
  /** Options keyed by costing model name. */
  costing_options?: CostingOptionsByModel
  /** Matching strategy. */
  shape_match?: TraceShapeMatch
  /** Search radius in meters. */
  search_radius?: number
  /** GPS accuracy in meters. */
  gps_accuracy?: number
  /** Distance in meters that starts a route break. */
  breakage_distance?: number
  /** Distance in meters beyond which points are interpolated or merged. */
  interpolation_distance?: number
  /** Penalty factor used while matching turns. */
  turn_penalty_factor?: number
  /** Use timestamps for time-aware matching. */
  use_timestamps?: boolean
  /** Correlation id echoed by Valhalla. */
  id?: string
  /** Valhalla accepts endpoint-specific extension fields. */
  [key: string]: unknown
}

/** /trace_route request. */
export type TraceRouteRequest = TraceRequest

/** /trace_route response. */
export type TraceRouteResponse = RouteResponse

/** /trace_attributes request. */
export interface TraceAttributesRequest extends TraceRequest {
  /** Attribute filter list and action. */
  filters?: {
    /** Attribute names to include or exclude. */
    attributes?: string[]
    /** Include or exclude listed attributes. */
    action?: AttributeFilterAction
  }
}

/** /trace_attributes response. */
export interface TraceAttributesResponse {
  /** Matched edge attributes. */
  edges?: Record<string, unknown>[]
  /** Per-input matched point data. */
  matched_points?: Record<string, unknown>[]
  /** Encoded matched shape. */
  shape?: string
  /** Correlation id echoed from the request. */
  id?: string
  /** Additional response values returned by Valhalla. */
  [key: string]: unknown
}
