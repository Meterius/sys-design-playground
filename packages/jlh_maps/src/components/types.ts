export enum GeoLocationType {
  Coords,
}

export type GeoLocation = {
  type: GeoLocationType.Coords
  coords: { lat: number; lng: number }
}
