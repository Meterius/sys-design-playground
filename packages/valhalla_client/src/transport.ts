import { ValhallaHttpError, ValhallaRequestError } from './errors'
import { HttpMethod, type RequestOptions, type ValhallaClientOptions } from './types/index'

export interface TransportOptions extends ValhallaClientOptions {
  defaultMethod: HttpMethod
  fetchImpl: typeof fetch
}

export function createTransportOptions(options: ValhallaClientOptions): TransportOptions {
  const fetchImpl = options.fetch ?? globalThis.fetch

  if (!fetchImpl) {
    throw new ValhallaRequestError('No fetch implementation is available')
  }

  return {
    ...options,
    defaultMethod: options.defaultMethod ?? HttpMethod.Post,
    fetchImpl,
  }
}

export function normalizeBaseUrl(baseUrl: string | URL): string {
  const value = baseUrl.toString().trim()
  return value.endsWith('/') ? value.slice(0, -1) : value
}

export function buildJsonUrl(baseUrl: string | URL, path: string, request?: unknown): string {
  const normalizedBaseUrl = normalizeBaseUrl(baseUrl)
  const url = new URL(path, `${normalizedBaseUrl}/`)

  if (request !== undefined) {
    url.searchParams.set('json', JSON.stringify(request))
  }

  return url.toString()
}

export async function requestJson<TResponse>(
  options: TransportOptions,
  path: string,
  body?: unknown,
  requestOptions: RequestOptions = {},
): Promise<TResponse> {
  const method = requestOptions.method ?? options.defaultMethod
  const controller = options.timeoutMs ? new AbortController() : undefined
  const timeout = controller
    ? setTimeout(() => controller.abort(), options.timeoutMs)
    : undefined

  const signal = requestOptions.signal ?? controller?.signal

  try {
    const url =
      method === HttpMethod.Get
        ? buildJsonUrl(options.baseUrl, path, body)
        : buildJsonUrl(options.baseUrl, path)
    const headers = new Headers(options.headers)

    if (options.clientId) {
      headers.set('X-Client-Id', options.clientId)
    }

    if (method === HttpMethod.Post) {
      headers.set('Content-Type', 'application/json')
    }

    if (requestOptions.headers) {
      new Headers(requestOptions.headers).forEach((value, key) => headers.set(key, value))
    }

    const init: RequestInit = {
      headers,
      method,
    }

    if (signal) {
      init.signal = signal
    }

    if (method === HttpMethod.Post && body !== undefined) {
      init.body = JSON.stringify(body)
    }

    const response = await options.fetchImpl(url, init)

    if (!response.ok) {
      throw new ValhallaHttpError(response, await parseErrorPayload(response))
    }

    return (await response.json()) as TResponse
  } finally {
    if (timeout !== undefined) {
      clearTimeout(timeout)
    }
  }
}

export async function requestRaw(
  options: TransportOptions,
  path: string,
  body?: unknown,
  requestOptions: RequestOptions = {},
): Promise<Response> {
  const method = requestOptions.method ?? HttpMethod.Get
  const url =
    method === HttpMethod.Get
      ? buildJsonUrl(options.baseUrl, path, body)
      : buildJsonUrl(options.baseUrl, path)
  const headers = new Headers(options.headers)

  if (options.clientId) {
    headers.set('X-Client-Id', options.clientId)
  }

  if (requestOptions.headers) {
    new Headers(requestOptions.headers).forEach((value, key) => headers.set(key, value))
  }

  const init: RequestInit = {
    headers,
    method,
  }

  if (requestOptions.signal) {
    init.signal = requestOptions.signal
  }

  if (method === HttpMethod.Post && body !== undefined) {
    headers.set('Content-Type', 'application/json')
    init.body = JSON.stringify(body)
  }

  const response = await options.fetchImpl(url, init)

  if (!response.ok) {
    throw new ValhallaHttpError(response, await parseErrorPayload(response))
  }

  return response
}

async function parseErrorPayload(response: Response): Promise<unknown> {
  try {
    return await response.clone().json()
  } catch {
    try {
      return { error: await response.text() }
    } catch {
      return undefined
    }
  }
}
