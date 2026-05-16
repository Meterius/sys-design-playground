import type {
  CostingModel,
  CostingOptionsByModel,
  LatLon,
  RouteResponse,
} from './common'

export enum TraceShapeMatch {
  EdgeWalk = 'edge_walk',
  MapSnap = 'map_snap',
  WalkOrSnap = 'walk_or_snap',
}

export enum AttributeFilterAction {
  Include = 'include',
  Exclude = 'exclude',
}

export interface TracePoint extends LatLon {
  time?: string
  accuracy?: number
  heading?: number
  [key: string]: unknown
}

export interface TraceRequest {
  shape: TracePoint[]
  costing: CostingModel
  costing_options?: CostingOptionsByModel
  shape_match?: TraceShapeMatch
  search_radius?: number
  gps_accuracy?: number
  breakage_distance?: number
  interpolation_distance?: number
  turn_penalty_factor?: number
  use_timestamps?: boolean
  id?: string
  [key: string]: unknown
}

export type TraceRouteRequest = TraceRequest
export type TraceRouteResponse = RouteResponse

export interface TraceAttributesRequest extends TraceRequest {
  filters?: {
    attributes?: string[]
    action?: AttributeFilterAction
  }
}

export interface TraceAttributesResponse {
  edges?: Record<string, unknown>[]
  matched_points?: Record<string, unknown>[]
  shape?: string
  id?: string
  [key: string]: unknown
}
