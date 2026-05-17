import type { ValhallaErrorPayload } from './types/index.js'

export class ValhallaHttpError extends Error {
  readonly response: Response
  readonly payload: ValhallaErrorPayload | unknown

  constructor(response: Response, payload: ValhallaErrorPayload | unknown) {
    const detail =
      typeof payload === 'object' && payload !== null && 'error' in payload
        ? String((payload as ValhallaErrorPayload).error)
        : response.statusText

    super(`Valhalla request failed: ${response.status} ${detail}`)
    this.name = 'ValhallaHttpError'
    this.response = response
    this.payload = payload
  }
}

export class ValhallaRequestError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'ValhallaRequestError'
  }
}
