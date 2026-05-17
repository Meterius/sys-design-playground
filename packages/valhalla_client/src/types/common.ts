import type { Position } from 'geojson'

/** HTTP method used by the Valhalla transport wrapper. */
export enum HttpMethod {
  Get = 'GET',
  Post = 'POST',
}

/** Per-request transport options. */
export interface RequestOptions {
  /** Override the client's default request method. */
  method?: HttpMethod
  /** AbortSignal passed to fetch. */
  signal?: AbortSignal
  /** Additional headers merged into the request. */
  headers?: HeadersInit
}

/** Options for constructing a ValhallaClient. */
export interface ValhallaClientOptions {
  /** Base URL of the Valhalla service, without endpoint path. */
  baseUrl: string | URL
  /** Optional client identifier sent as X-Client-Id. */
  clientId?: string
  /** Default method for JSON endpoints. Defaults to POST. */
  defaultMethod?: HttpMethod
  /** Custom fetch implementation for tests or non-standard runtimes. */
  fetch?: typeof fetch
  /** Headers applied to every request. */
  headers?: HeadersInit
  /** Optional request timeout in milliseconds. */
  timeoutMs?: number
}

/** Error payload returned by Valhalla for failed requests. */
export interface ValhallaErrorPayload {
  /** Human-readable error message. */
  error?: string
  /** Valhalla internal error code when provided. */
  error_code?: number
  /** HTTP or service status. */
  status?: number
  /** Alternate status code field used by some responses. */
  status_code?: number
  /** Human-readable status detail. */
  status_message?: string
}

/** Warning returned inside successful Valhalla responses. */
export interface ValhallaWarning {
  /** Valhalla warning code. */
  code: number
  /** Human-readable warning text. */
  text: string
}

/** Latitude/longitude pair using Valhalla's lat/lon property names. */
export interface LatLon {
  /** Latitude in decimal degrees. */
  lat: number
  /** Longitude in decimal degrees. */
  lon: number
}

/** GeoJSON position, normally [lon, lat]. */
export type GeoJsonPosition = Position

/** Valhalla costing model names.
 *
 * @see https://valhalla.github.io/valhalla/api/turn-by-turn/api-reference/#costing-models
 */
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

/** How Valhalla treats a waypoint during routing.
 *
 * @see https://valhalla.github.io/valhalla/api/turn-by-turn/api-reference/#locations
 */
export enum LocationType {
  Break = 'break',
  Through = 'through',
  Via = 'via',
  BreakThrough = 'break_through',
}

/** Preferred side-of-street behavior for a location. */
export enum PreferredSide {
  Same = 'same',
  Opposite = 'opposite',
  Either = 'either',
}

/** Side of street returned for correlated locations. */
export enum SideOfStreet {
  Left = 'left',
  Right = 'right',
}

/** Road classes accepted by location search filters. */
export enum RoadClass {
  Motorway = 'motorway',
  Trunk = 'trunk',
  Primary = 'primary',
  Secondary = 'secondary',
  Tertiary = 'tertiary',
  Unclassified = 'unclassified',
  Residential = 'residential',
  ServiceOther = 'service_other',
}

/** Filters used while correlating a location to candidate graph edges. */
export interface SearchFilter {
  /** Exclude roads marked as tunnels. */
  exclude_tunnel?: boolean
  /** Exclude roads marked as bridges. */
  exclude_bridge?: boolean
  /** Exclude toll roads. */
  exclude_toll?: boolean
  /** Exclude ferries. */
  exclude_ferry?: boolean
  /** Exclude ramps and link roads. */
  exclude_ramp?: boolean
  /** Exclude closures during location correlation. */
  exclude_closures?: boolean
  /** Lowest road class allowed during candidate search. */
  min_road_class?: RoadClass
  /** Highest road class allowed during candidate search. */
  max_road_class?: RoadClass
  /** Indoor or layered floor level to consider. */
  level?: number
}

/** A route, matrix, isochrone, locate, or expansion input location.
 *
 * @see https://valhalla.github.io/valhalla/api/turn-by-turn/api-reference/#locations
 */
export interface ValhallaLocation extends LatLon {
  /** Waypoint behavior. Defaults to break in Valhalla. */
  type?: LocationType
  /** Place or business name carried through request/response. */
  name?: string
  /** Street hint used during correlation where supported. */
  street?: string
  /** City metadata carried through request/response. */
  city?: string
  /** State metadata carried through request/response. */
  state?: string
  /** Postal code metadata carried through request/response. */
  postal_code?: string
  /** Country metadata carried through request/response. */
  country?: string
  /** Preferred departure heading in clockwise degrees from north. */
  heading?: number
  /** Allowed heading difference in degrees. */
  heading_tolerance?: number
  /** Snap-to-intersection tolerance in meters. */
  node_snap_tolerance?: number
  /** Minimum reachable nodes required for candidate edges. */
  minimum_reachability?: number
  /** Candidate search radius in meters. */
  radius?: number
  /** Whether candidate edges should be ranked by correlation quality. */
  rank_candidates?: boolean
  /** Preferred side of street for arrival/departure. */
  preferred_side?: PreferredSide
  /** Display latitude used for side-of-street while lat/lon remain routing coordinates. */
  display_lat?: number
  /** Display longitude used with display_lat. */
  display_lon?: number
  /** Candidate-edge filters applied during location correlation. */
  search_filter?: SearchFilter
  /** Valhalla forwards additional location metadata it understands. */
  [key: string]: unknown
}

/** Valhalla date/time behavior. */
export enum DateTimeType {
  CurrentDeparture = 0,
  SpecifiedDeparture = 1,
  SpecifiedArrival = 2,
  Invariant = 3,
}

/** Time constraint for routing requests.
 *
 * @see https://valhalla.github.io/valhalla/api/turn-by-turn/api-reference/#other-request-options
 */
export interface DateTime {
  /** Departure/arrival interpretation. */
  type: DateTimeType
  /** Local time value, usually YYYY-MM-DDTHH:mm. */
  value?: string
}

/** Distance units used by route, matrix, and isochrone requests. */
export enum Units {
  Kilometers = 'kilometers',
  Miles = 'miles',
}

/** Route shape encoding requested from Valhalla. */
export enum ShapeFormat {
  Polyline6 = 'polyline6',
  Polyline5 = 'polyline5',
  GeoJson = 'geojson',
  NoShape = 'no_shape',
}

/** Isochrone contour metric. */
export enum ContourUnit {
  Time = 'time',
  Distance = 'distance',
}

/** Travel mode returned on maneuvers. */
export enum TravelMode {
  Drive = 'drive',
  Pedestrian = 'pedestrian',
  Bicycle = 'bicycle',
  Transit = 'transit',
  Bikeshare = 'bikeshare',
}

/** Travel type returned on maneuvers for the active travel mode. */
export enum TravelType {
  Car = 'car',
  Motorcycle = 'motorcycle',
  MotorScooter = 'motor_scooter',
  Truck = 'truck',
  Bus = 'bus',
  Foot = 'foot',
  Wheelchair = 'wheelchair',
  Road = 'road',
  Hybrid = 'hybrid',
  Cross = 'cross',
  Mountain = 'mountain',
  Tram = 'tram',
  Metro = 'metro',
  Rail = 'rail',
  Ferry = 'ferry',
  CableCar = 'cable_car',
  Gondola = 'gondola',
  Funicular = 'funicular',
}

/** Bike-share maneuver type returned when travel_mode is bikeshare. */
export enum BikeShareManeuverType {
  NoneAction = 'NoneAction',
  RentBikeAtBikeShare = 'RentBikeAtBikeShare',
  ReturnBikeAtBikeShare = 'ReturnBikeAtBikeShare',
}

/** Lane guidance bitmask values.
 *
 * @see https://valhalla.github.io/valhalla/api/turn-by-turn/api-reference/#trip-legs-and-maneuvers
 */
export enum LaneDirection {
  Empty = 0,
  None = 1,
  Through = 2,
  SharpLeft = 4,
  Left = 8,
  SlightLeft = 16,
  SlightRight = 32,
  Right = 64,
  SharpRight = 128,
  Reverse = 256,
  MergeToLeft = 512,
  MergeToRight = 1024,
}

/** Odin maneuver type codes.
 *
 * @see https://valhalla.github.io/valhalla/api/turn-by-turn/api-reference/#trip-legs-and-maneuvers
 */
export enum ManeuverType {
  None = 0,
  Start = 1,
  StartRight = 2,
  StartLeft = 3,
  Destination = 4,
  DestinationRight = 5,
  DestinationLeft = 6,
  Becomes = 7,
  Continue = 8,
  SlightRight = 9,
  Right = 10,
  SharpRight = 11,
  UturnRight = 12,
  UturnLeft = 13,
  SharpLeft = 14,
  Left = 15,
  SlightLeft = 16,
  RampStraight = 17,
  RampRight = 18,
  RampLeft = 19,
  ExitRight = 20,
  ExitLeft = 21,
  StayStraight = 22,
  StayRight = 23,
  StayLeft = 24,
  Merge = 25,
  RoundaboutEnter = 26,
  RoundaboutExit = 27,
  FerryEnter = 28,
  FerryExit = 29,
  Transit = 30,
  TransitTransfer = 31,
  TransitRemainOn = 32,
  TransitConnectionStart = 33,
  TransitConnectionTransfer = 34,
  TransitConnectionDestination = 35,
  PostTransitConnectionDestination = 36,
  MergeRight = 37,
  MergeLeft = 38,
  ElevatorEnter = 39,
  StepsEnter = 40,
  EscalatorEnter = 41,
  BuildingEnter = 42,
  BuildingExit = 43,
}

/** Maneuver sign text element. */
export interface ManeuverSignElement {
  /** Text shown on the sign. */
  text: string
  /** Frequency of this sign element within consecutive signs. */
  consecutive_count?: number
}

/** Interchange guide sign elements for a maneuver. */
export interface ManeuverSign {
  /** Exit number elements, such as 91B. */
  exit_number_elements?: ManeuverSignElement[]
  /** Exit branch elements, such as I 95 North. */
  exit_branch_elements?: ManeuverSignElement[]
  /** Exit toward elements, such as New York. */
  exit_toward_elements?: ManeuverSignElement[]
  /** Exit name elements, such as Gettysburg Pike. */
  exit_name_elements?: ManeuverSignElement[]
}

/** Lane-level guidance for a maneuver. */
export interface ManeuverLane {
  /** Bitmask of all possible lane directions. */
  directions: LaneDirection
  /** Bitmask of lane directions valid at the start of the maneuver. */
  valid?: LaneDirection
  /** Bitmask of lane directions active for continuing along the route. */
  active?: LaneDirection
}

export type CostingOptions = Record<string, unknown>
export type CostingOptionsByModel = Partial<Record<CostingModel, CostingOptions>> &
  Record<string, CostingOptions | undefined>

/** Common request shape for endpoints that route from ordered locations. */
export interface BaseRoutingRequest {
  /** Ordered input locations. */
  locations: ValhallaLocation[]
  /** Costing model used for path finding. */
  costing: CostingModel
  /** Options keyed by costing model name. */
  costing_options?: CostingOptionsByModel
  /** Output distance units. */
  units?: Units
  /** Narrative language tag. */
  language?: string
  /** Options for narrative, shape, and guidance output. */
  directions_options?: Record<string, unknown>
  /** Departure or arrival time constraints. */
  date_time?: DateTime
  /** Correlation id echoed by Valhalla. */
  id?: string
  /** Valhalla accepts endpoint-specific extension fields. */
  [key: string]: unknown
}

/** Location object returned inside a trip response. */
export interface TripLocation extends ValhallaLocation {
  /** Index of the original request location. */
  original_index?: number
  /** Side of street selected by the routed path. */
  side_of_street?: SideOfStreet
}

/** Summary for a full trip or one trip leg. */
export interface TripSummary {
  /** True if the path includes time restrictions. */
  has_time_restrictions?: boolean
  /** True if any toll road is used. */
  has_toll?: boolean
  /** True if any highway is used. */
  has_highway?: boolean
  /** True if any ferry is used. */
  has_ferry?: boolean
  /** Minimum latitude of the route bounding box. */
  min_lat?: number
  /** Minimum longitude of the route bounding box. */
  min_lon?: number
  /** Maximum latitude of the route bounding box. */
  max_lat?: number
  /** Maximum longitude of the route bounding box. */
  max_lon?: number
  /** Estimated elapsed time in seconds. */
  time: number
  /** Distance in requested units. */
  length: number
  /** Internal path cost. */
  cost: number
  /** Additional summary values returned by Valhalla. */
  [key: string]: unknown
}

/** One navigation instruction within a trip leg. */
export interface Maneuver {
  /** Numeric maneuver type code. */
  type: ManeuverType
  /** Written maneuver instruction. */
  instruction: string
  /** Short spoken transition instruction. */
  verbal_succinct_transition_instruction?: string
  /** Spoken instruction before the maneuver. */
  verbal_pre_transition_instruction?: string
  /** Spoken instruction after the maneuver. */
  verbal_post_transition_instruction?: string
  /** Street names consistent along the maneuver. */
  street_names?: string[]
  /** Street names at the beginning of the maneuver. */
  begin_street_names?: string[]
  /** Direction of travel before the maneuver, clockwise degrees from north. */
  bearing_before?: number
  /** Direction of travel after the maneuver, clockwise degrees from north. */
  bearing_after?: number
  /** Estimated maneuver time in seconds. */
  time: number
  /** Maneuver distance in requested units. */
  length: number
  /** Internal maneuver cost. */
  cost: number
  /** Start index into the decoded leg shape. */
  begin_shape_index: number
  /** End index into the decoded leg shape. */
  end_shape_index: number
  /** Travel mode active during this maneuver. */
  travel_mode?: TravelMode
  /** Travel type active during this maneuver. */
  travel_type?: TravelType
  /** Bike-share transition type when travel_mode is bikeshare. */
  bss_maneuver_type?: BikeShareManeuverType
  /** Interchange sign information. */
  sign?: ManeuverSign
  /** Lane-level guidance. */
  lanes?: ManeuverLane[]
  /** True when any part of the maneuver is tolled. */
  toll?: boolean
  /** True when a highway is encountered. */
  highway?: boolean
  /** True when unpaved or rough pavement is encountered. */
  rough?: boolean
  /** True when a gate is encountered. */
  gate?: boolean
  /** True when a ferry is encountered. */
  ferry?: boolean
  /** Additional maneuver values returned by Valhalla. */
  [key: string]: unknown
}

/** One leg between break or break_through locations. */
export interface TripLeg {
  /** Maneuvers generated for this leg. */
  maneuvers?: Maneuver[]
  /** Leg-level summary. */
  summary: TripSummary
  /** Encoded polyline shape with six decimal digits by default. */
  shape?: string
  /** Alternate shape field used by some Valhalla configurations. */
  encoded_shape?: string
  /** Additional leg values returned by Valhalla. */
  [key: string]: unknown
}

/** Valhalla trip object returned by route-like endpoints. */
export interface Trip {
  /** Correlated locations used in the route. */
  locations: TripLocation[]
  /** Route legs. */
  legs: TripLeg[]
  /** Trip-level summary. */
  summary: TripSummary
  /** Human-readable status message. */
  status_message: string
  /** Valhalla trip status code. */
  status: number
  /** Response distance units. */
  units?: Units
  /** Narrative language used for instructions. */
  language?: string
  /** Non-fatal warnings generated by Valhalla. */
  warnings?: ValhallaWarning[]
}

/** Route-like endpoint response. */
export interface RouteResponse {
  /** Routed trip. */
  trip: Trip
  /** Correlation id echoed from the request. */
  id?: string
  /** Alternative routes when requested. */
  alternates?: RouteResponse[]
}
