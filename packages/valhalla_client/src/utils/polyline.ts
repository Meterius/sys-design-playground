import type { Position } from 'geojson'
import type { LatLon } from '../types/index.js'

/**
 * Decodes an encoded route shape into Valhalla `{ lat, lon }` coordinates.
 *
 * Valhalla route shapes use the Google Encoded Polyline Algorithm with six
 * decimal digits of precision by default (`polyline6`). Each coordinate is
 * encoded as a delta from the previous coordinate, scaled to an integer,
 * zig-zag encoded so negative deltas become unsigned integers, then split into
 * 5-bit chunks with a continuation bit and an ASCII offset of 63.
 *
 * References:
 * - Valhalla shape decoding: https://valhalla.github.io/valhalla/decoding/
 * - Google Encoded Polyline Algorithm Format:
 *   https://developers.google.com/maps/documentation/utilities/polylinealgorithm
 */
export function decodePolyline(encoded: string, precision = 6): LatLon[] {
  const factor = 10 ** precision
  const coordinates: LatLon[] = []
  let index = 0
  let lat = 0
  let lon = 0

  while (index < encoded.length) {
    const latitudeResult = decodeSignedValue(encoded, index)
    index = latitudeResult.nextIndex
    lat += latitudeResult.value

    const longitudeResult = decodeSignedValue(encoded, index)
    index = longitudeResult.nextIndex
    lon += longitudeResult.value

    coordinates.push({
      lat: lat / factor,
      lon: lon / factor,
    })
  }

  return coordinates
}

/**
 * Decodes an encoded route shape into GeoJSON positions in `[lon, lat]` order.
 */
export function decodePolylineToPositions(encoded: string, precision = 6): Position[] {
  return decodePolyline(encoded, precision).map(({ lat, lon }) => [lon, lat])
}

/**
 * Encodes Valhalla `{ lat, lon }` coordinates into an encoded polyline string.
 */
export function encodePolyline(coordinates: LatLon[], precision = 6): string {
  const factor = 10 ** precision
  let previousLat = 0
  let previousLon = 0
  let encoded = ''

  for (const coordinate of coordinates) {
    const lat = Math.round(coordinate.lat * factor)
    const lon = Math.round(coordinate.lon * factor)

    encoded += encodeSignedValue(lat - previousLat)
    encoded += encodeSignedValue(lon - previousLon)

    previousLat = lat
    previousLon = lon
  }

  return encoded
}

/**
 * Encodes GeoJSON positions in `[lon, lat]` order into an encoded polyline string.
 */
export function encodePositionsToPolyline(positions: Position[], precision = 6): string {
  return encodePolyline(
    positions.map((position) => {
      const [lon = 0, lat = 0] = position
      return { lat, lon }
    }),
    precision,
  )
}

function decodeSignedValue(encoded: string, startIndex: number) {
  let result = 0
  let shift = 0
  let index = startIndex
  let byte: number

  do {
    if (index >= encoded.length) {
      throw new Error('Invalid polyline encoding')
    }

    byte = encoded.charCodeAt(index) - 63
    index += 1
    result |= (byte & 0x1f) << shift
    shift += 5
  } while (byte >= 0x20)

  return {
    nextIndex: index,
    value: result & 1 ? ~(result >> 1) : result >> 1,
  }
}

function encodeSignedValue(value: number) {
  let shifted = value < 0 ? ~(value << 1) : value << 1
  let output = ''

  while (shifted >= 0x20) {
    output += String.fromCharCode((0x20 | (shifted & 0x1f)) + 63)
    shifted >>= 5
  }

  output += String.fromCharCode(shifted + 63)
  return output
}
