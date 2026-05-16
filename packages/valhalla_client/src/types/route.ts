import type { Geometry } from 'geojson'
import type { BaseRoutingRequest, RouteResponse, ShapeFormat, ValhallaLocation } from './common'

export interface RouteRequest extends BaseRoutingRequest {
  alternates?: number
  exclude_locations?: ValhallaLocation[]
  exclude_polygons?: Geometry[]
  shape_format?: ShapeFormat
}

export type { RouteResponse }
