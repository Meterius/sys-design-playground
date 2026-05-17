import type { BaseRoutingRequest, RouteResponse } from './common.js'

/** Optimized route request that lets Valhalla reorder intermediate locations.
 *
 * @see https://valhalla.github.io/valhalla/api/optimized/api-reference/
 */
export type OptimizedRouteRequest = BaseRoutingRequest

/** Optimized route response. */
export type OptimizedRouteResponse = RouteResponse
