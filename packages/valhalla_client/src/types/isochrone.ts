import type { Feature, FeatureCollection, GeoJsonProperties, Geometry } from 'geojson'
import type {
  CostingModel,
  CostingOptionsByModel,
  Units,
  ValhallaLocation,
  ValhallaWarning,
} from './common'

export interface Contour {
  time?: number
  distance?: number
  color?: string
}

export interface IsochroneRequest {
  locations: ValhallaLocation[]
  costing: CostingModel
  contours: Contour[]
  costing_options?: CostingOptionsByModel
  polygons?: boolean
  denoise?: number
  generalize?: number
  show_locations?: boolean
  units?: Units
  id?: string
  [key: string]: unknown
}

export type IsochroneFeature = Feature<Geometry, GeoJsonProperties>

export interface IsochroneResponse extends FeatureCollection<Geometry, GeoJsonProperties> {
  id?: string
  warnings?: ValhallaWarning[]
}
