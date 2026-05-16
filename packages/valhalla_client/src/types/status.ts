export interface StatusResponse {
  version?: string
  tileset_last_modified?: number
  available_actions?: string[]
  has_tiles?: boolean
  bbox?: [number, number, number, number]
  [key: string]: unknown
}
