import type {
  CostingModel,
  CostingOptionsByModel,
  TripLocation,
  Units,
  ValhallaLocation,
} from './common'

export interface MatrixRequest {
  sources: ValhallaLocation[]
  targets: ValhallaLocation[]
  costing: CostingModel
  costing_options?: CostingOptionsByModel
  units?: Units
  id?: string
  [key: string]: unknown
}

export interface MatrixCell {
  distance?: number
  time?: number
  to_index?: number
  from_index?: number
  date_time?: string
  [key: string]: unknown
}

export interface MatrixResponse {
  sources?: TripLocation[]
  targets?: TripLocation[]
  sources_to_targets: MatrixCell[][]
  units?: Units
  id?: string
  [key: string]: unknown
}
