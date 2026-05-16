import type { Position } from 'geojson'

export enum HttpMethod {
  Get = 'GET',
  Post = 'POST',
}

export interface RequestOptions {
  method?: HttpMethod
  signal?: AbortSignal
  headers?: HeadersInit
}

export interface ValhallaClientOptions {
  baseUrl: string | URL
  clientId?: string
  defaultMethod?: HttpMethod
  fetch?: typeof fetch
  headers?: HeadersInit
  timeoutMs?: number
}

export interface ValhallaErrorPayload {
  error?: string
  error_code?: number
  status?: number
  status_code?: number
  status_message?: string
}

export interface ValhallaWarning {
  code: number
  text: string
}

export interface LatLon {
  lat: number
  lon: number
}

export enum CostingModel {
  Auto = 'auto',
  AutoShorter = 'auto_shorter',
  Bicycle = 'bicycle',
  Bikeshare = 'bikeshare',
  Bus = 'bus',
  Hov = 'hov',
  MotorScooter = 'motor_scooter',
  Motorcycle = 'motorcycle',
  Multimodal = 'multimodal',
  Pedestrian = 'pedestrian',
  Taxi = 'taxi',
  Truck = 'truck',
}

export enum LocationType {
  Break = 'break',
  Through = 'through',
  Via = 'via',
  BreakThrough = 'break_through',
}

export enum PreferredSide {
  Same = 'same',
  Opposite = 'opposite',
  Either = 'either',
}

export interface SearchFilter {
  exclude_tunnel?: boolean
  exclude_bridge?: boolean
  exclude_ramp?: boolean
  exclude_closures?: boolean
  min_road_class?: string
  max_road_class?: string
  level?: number
}

export interface ValhallaLocation extends LatLon {
  type?: LocationType
  name?: string
  street?: string
  city?: string
  state?: string
  postal_code?: string
  country?: string
  heading?: number
  heading_tolerance?: number
  node_snap_tolerance?: number
  minimum_reachability?: number
  radius?: number
  rank_candidates?: boolean
  preferred_side?: PreferredSide
  display_lat?: number
  display_lon?: number
  search_filter?: SearchFilter
  [key: string]: unknown
}

export enum DateTimeType {
  CurrentDeparture = 0,
  SpecifiedDeparture = 1,
  SpecifiedArrival = 2,
  Invariant = 3,
}

export interface DateTime {
  type: DateTimeType
  value?: string
}

export enum Units {
  Kilometers = 'kilometers',
  Miles = 'miles',
}

export enum ShapeFormat {
  Polyline6 = 'polyline6',
  Polyline5 = 'polyline5',
  GeoJson = 'geojson',
  NoShape = 'no_shape',
}

export enum ContourUnit {
  Time = 'time',
  Distance = 'distance',
}

export type CostingOptions = Record<string, unknown>
export type CostingOptionsByModel = Partial<Record<CostingModel, CostingOptions>> &
  Record<string, CostingOptions | undefined>

export interface BaseRoutingRequest {
  locations: ValhallaLocation[]
  costing: CostingModel
  costing_options?: CostingOptionsByModel
  units?: Units
  language?: string
  directions_options?: Record<string, unknown>
  date_time?: DateTime
  id?: string
  [key: string]: unknown
}

export interface TripLocation extends ValhallaLocation {
  original_index?: number
  side_of_street?: string
}

export interface TripSummary {
  has_time_restrictions?: boolean
  has_toll?: boolean
  has_highway?: boolean
  has_ferry?: boolean
  min_lat?: number
  min_lon?: number
  max_lat?: number
  max_lon?: number
  time: number
  length: number
  cost: number
  [key: string]: unknown
}

export interface Maneuver {
  type: number
  instruction: string
  verbal_succinct_transition_instruction?: string
  verbal_pre_transition_instruction?: string
  verbal_post_transition_instruction?: string
  street_names?: string[]
  begin_street_names?: string[]
  bearing_before?: number
  bearing_after?: number
  time: number
  length: number
  cost: number
  begin_shape_index: number
  end_shape_index: number
  travel_mode?: string
  travel_type?: string
  [key: string]: unknown
}

export interface TripLeg {
  maneuvers?: Maneuver[]
  summary: TripSummary
  shape?: string
  encoded_shape?: string
  [key: string]: unknown
}

export interface Trip {
  locations: TripLocation[]
  legs: TripLeg[]
  summary: TripSummary
  status_message: string
  status: number
  units?: Units
  language?: string
  warnings?: ValhallaWarning[]
}

export interface RouteResponse {
  trip: Trip
  id?: string
  alternates?: RouteResponse[]
}
