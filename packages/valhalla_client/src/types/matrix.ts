import type {
  CostingModel,
  CostingOptionsByModel,
  TripLocation,
  Units,
  ValhallaLocation,
} from './common.js'

/** Matrix request for source-to-target travel times and distances.
 *
 * @see https://valhalla.github.io/valhalla/api/matrix/api-reference/
 */
export interface MatrixRequest {
  /** Origin locations. */
  sources: ValhallaLocation[]
  /** Destination locations. */
  targets: ValhallaLocation[]
  /** Costing model used for each pair. */
  costing: CostingModel
  /** Options keyed by costing model name. */
  costing_options?: CostingOptionsByModel
  /** Distance units for the response. */
  units?: Units
  /** Correlation id echoed by Valhalla. */
  id?: string
  /** Valhalla accepts endpoint-specific extension fields. */
  [key: string]: unknown
}

/** One source-to-target matrix entry. */
export interface MatrixCell {
  /** Distance between source and target in requested units. */
  distance?: number
  /** Travel time in seconds. */
  time?: number
  /** Target index for this cell. */
  to_index?: number
  /** Source index for this cell. */
  from_index?: number
  /** Time at the matrix cell when time-dependent routing is used. */
  date_time?: string
  /** Additional matrix-cell values returned by Valhalla. */
  [key: string]: unknown
}

/** Matrix response from /sources_to_targets. */
export interface MatrixResponse {
  /** Correlated source locations. */
  sources?: TripLocation[]
  /** Correlated target locations. */
  targets?: TripLocation[]
  /** Row-major source-to-target result cells. */
  sources_to_targets: MatrixCell[][]
  /** Distance units used in the response. */
  units?: Units
  /** Correlation id echoed from the request. */
  id?: string
  /** Additional response values returned by Valhalla. */
  [key: string]: unknown
}
