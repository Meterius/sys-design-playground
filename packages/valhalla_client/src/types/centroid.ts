import type { BaseRoutingRequest, LatLon, Trip, ValhallaWarning } from './common'

export type CentroidRequest = BaseRoutingRequest

export interface CentroidResponse {
  id?: string
  centroid?: LatLon
  trip?: Trip
  warnings?: ValhallaWarning[]
  [key: string]: unknown
}
