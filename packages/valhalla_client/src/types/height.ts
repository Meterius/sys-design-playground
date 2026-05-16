import type { LatLon } from './common'

export interface HeightRequest {
  shape: LatLon[]
  range?: boolean
  encoded_polyline?: string
  id?: string
  [key: string]: unknown
}

export interface HeightPoint extends LatLon {
  height: number | null
}

export interface HeightResponse {
  shape?: HeightPoint[]
  height?: Array<number | null>
  range_height?: Array<[number, number | null]>
  id?: string
  [key: string]: unknown
}
