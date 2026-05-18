export enum GeoLocationType {
  Coords,
}

export enum RouteMode {
  Car = 'car',
  Bicycle = 'bicycle',
  Foot = 'foot',
}

export type GeoLocation = {
  type: GeoLocationType.Coords
  coords: { lat: number; lng: number }
}
