import type {
  CentroidRequest,
  CentroidResponse,
  ExpansionRequest,
  ExpansionResponse,
  HeightRequest,
  HeightResponse,
  IsochroneRequest,
  IsochroneResponse,
  LocateRequest,
  LocateResponse,
  MatrixRequest,
  MatrixResponse,
  OptimizedRouteRequest,
  OptimizedRouteResponse,
  RequestOptions,
  RouteRequest,
  RouteResponse,
  StatusResponse,
  TileRequest,
  TraceAttributesRequest,
  TraceAttributesResponse,
  TraceRouteRequest,
  TraceRouteResponse,
  ValhallaClientOptions,
} from './types/index.js'
import { HttpMethod } from './types/index.js'
import {
  buildJsonUrl,
  createTransportOptions,
  normalizeBaseUrl,
  requestJson,
  requestRaw,
  type TransportOptions,
} from './transport.js'

export class ValhallaClient {
  readonly baseUrl: string
  private readonly transportOptions: TransportOptions

  constructor(options: ValhallaClientOptions) {
    this.baseUrl = normalizeBaseUrl(options.baseUrl)
    this.transportOptions = createTransportOptions({
      ...options,
      baseUrl: this.baseUrl,
    })
  }

  route(request: RouteRequest, options?: RequestOptions): Promise<RouteResponse> {
    return this.query('/route', request, options)
  }

  optimizedRoute(
    request: OptimizedRouteRequest,
    options?: RequestOptions,
  ): Promise<OptimizedRouteResponse> {
    return this.query('/optimized_route', request, options)
  }

  matrix(request: MatrixRequest, options?: RequestOptions): Promise<MatrixResponse> {
    return this.query('/sources_to_targets', request, options)
  }

  isochrone(request: IsochroneRequest, options?: RequestOptions): Promise<IsochroneResponse> {
    return this.query('/isochrone', request, options)
  }

  traceRoute(request: TraceRouteRequest, options?: RequestOptions): Promise<TraceRouteResponse> {
    return this.query('/trace_route', request, options)
  }

  traceAttributes(
    request: TraceAttributesRequest,
    options?: RequestOptions,
  ): Promise<TraceAttributesResponse> {
    return this.query('/trace_attributes', request, options)
  }

  locate(request: LocateRequest, options?: RequestOptions): Promise<LocateResponse[]> {
    return this.query('/locate', request, options)
  }

  height(request: HeightRequest, options?: RequestOptions): Promise<HeightResponse> {
    return this.query('/height', request, options)
  }

  expansion(request: ExpansionRequest, options?: RequestOptions): Promise<ExpansionResponse> {
    return this.query('/expansion', request, options)
  }

  status(options?: Omit<RequestOptions, 'method'>): Promise<StatusResponse> {
    return this.query('/status', undefined, { ...options, method: HttpMethod.Get })
  }

  centroid(request: CentroidRequest, options?: RequestOptions): Promise<CentroidResponse> {
    return this.query('/centroid', request, options)
  }

  tileUrl(request: TileRequest): string {
    return buildJsonUrl(this.baseUrl, '/tile', request)
  }

  tile(request: TileRequest, options?: RequestOptions): Promise<Response> {
    return requestRaw(this.transportOptions, '/tile', request, {
      ...options,
      method: options?.method ?? HttpMethod.Get,
    })
  }

  query<TResponse>(
    path: string,
    request?: unknown,
    options?: RequestOptions,
  ): Promise<TResponse> {
    return requestJson<TResponse>(this.transportOptions, path, request, options)
  }
}
