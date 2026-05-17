export interface DEMData {
  uid?: unknown
  stride: number
  dim: number
  min: number
  max: number
  redFactor: number
  greenFactor: number
  blueFactor: number
  baseShift: number
  data: ArrayLike<number>
}
