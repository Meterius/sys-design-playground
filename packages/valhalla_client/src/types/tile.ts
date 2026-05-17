/** Tile request for Valhalla debug vector tiles.
 *
 * @see https://valhalla.github.io/valhalla/api/tile/api-reference/
 */
export interface TileRequest {
  /** XYZ tile coordinates. Strings are allowed for URL templates such as {z}/{x}/{y}. */
  tile: {
    /** X tile coordinate or template token. */
    x: number | string
    /** Y tile coordinate or template token. */
    y: number | string
    /** Z tile coordinate or template token. */
    z: number | string
  }
  /** Include additional tile properties when true. */
  verbose?: boolean
  /** Correlation id echoed by Valhalla. */
  id?: string
  /** Valhalla accepts endpoint-specific extension fields. */
  [key: string]: unknown
}
