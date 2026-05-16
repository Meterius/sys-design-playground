import type { FeatureCollection, GeoJsonProperties, Geometry } from 'geojson'
import type {
  CostingModel,
  CostingOptionsByModel,
  ValhallaLocation,
  ValhallaWarning,
} from './common'

export interface ExpansionRequest {
  locations: ValhallaLocation[]
  costing: CostingModel
  costing_options?: CostingOptionsByModel
  expansion_properties?: string[]
  skip_opposites?: boolean
  id?: string
  [key: string]: unknown
}

export interface ExpansionResponse extends FeatureCollection<Geometry, GeoJsonProperties> {
  id?: string
  warnings?: ValhallaWarning[]
}
