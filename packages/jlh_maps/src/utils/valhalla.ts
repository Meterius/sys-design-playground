import { LngLatBounds } from 'maplibre-gl'
import { decodePolylineToPositions, type Trip } from 'valhalla_client'

const isFiniteNumber = (value: unknown): value is number =>
  typeof value === 'number' && Number.isFinite(value)

export const getTripBounds = (trip: Trip) => {
  const { min_lon, min_lat, max_lon, max_lat } = trip.summary

  if (
    isFiniteNumber(min_lon) &&
    isFiniteNumber(min_lat) &&
    isFiniteNumber(max_lon) &&
    isFiniteNumber(max_lat)
  ) {
    return new LngLatBounds([min_lon, min_lat], [max_lon, max_lat])
  }

  const bounds = new LngLatBounds()
  let hasCoordinates = false

  for (const leg of trip.legs) {
    const shape = leg.shape ?? leg.encoded_shape
    if (!shape) continue

    for (const [lng, lat] of decodePolylineToPositions(shape)) {
      if (!isFiniteNumber(lng) || !isFiniteNumber(lat)) continue

      bounds.extend([lng, lat])
      hasCoordinates = true
    }
  }

  for (const location of trip.locations) {
    if (!isFiniteNumber(location.lon) || !isFiniteNumber(location.lat)) continue

    bounds.extend([location.lon, location.lat])
    hasCoordinates = true
  }

  return hasCoordinates ? bounds : null
}
