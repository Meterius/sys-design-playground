import {
  type CustomLayerInterface,
  type CustomRenderMethodInput,
  type Map as MapLibreMap,
} from 'maplibre-gl'

type TileKey = string

interface TileCoord {
  z: number
  x: number
  y: number
}

export class AppLayer implements CustomLayerInterface {
  id = 'app-layer'
  type = 'custom' as const
  renderingMode = '3d' as const

  private map!: MapLibreMap

  constructor() {
  }

  onAdd(map: MapLibreMap, gl: WebGLRenderingContext | WebGL2RenderingContext): void {
    this.map = map
  }

  render(_gl: WebGLRenderingContext | WebGL2RenderingContext, options: CustomRenderMethodInput): void {
  }

  onRemove(): void {
  }
}
