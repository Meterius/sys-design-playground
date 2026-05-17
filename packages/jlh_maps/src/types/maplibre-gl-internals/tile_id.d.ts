export class CanonicalTileID {
  constructor(z: number, x: number, y: number)

  z: number
  x: number
  y: number
  key: string

  toString(): string
}

export class OverscaledTileID {
  constructor(overscaledZ: number, wrap: number, z: number, x: number, y: number)

  overscaledZ: number
  wrap: number
  canonical: CanonicalTileID
  key: string

  toString(): string
}
