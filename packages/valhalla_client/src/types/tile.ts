export interface TileRequest {
  tile: {
    x: number | string
    y: number | string
    z: number | string
  }
  verbose?: boolean
  id?: string
  [key: string]: unknown
}
