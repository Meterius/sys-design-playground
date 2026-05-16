import type {
  CostingModel,
  CostingOptionsByModel,
  ValhallaLocation,
  ValhallaWarning,
} from './common'

export interface LocateRequest {
  locations: ValhallaLocation[]
  costing?: CostingModel
  costing_options?: CostingOptionsByModel
  verbose?: boolean
  id?: string
  [key: string]: unknown
}

export interface LocateEdge {
  correlated_lat?: number
  correlated_lon?: number
  percent_along?: number
  distance?: number
  way_id?: number
  id?: string
  names?: string[]
  [key: string]: unknown
}

export interface LocateResponse {
  input_lat?: number
  input_lon?: number
  nodes?: Record<string, unknown>[]
  edges?: LocateEdge[]
  warnings?: ValhallaWarning[]
  id?: string
  [key: string]: unknown
}
