# valhalla_client

Typed TypeScript client for Valhalla HTTP APIs.

## Usage

```ts
import { CostingModel, ValhallaClient } from 'valhalla_client'

const valhalla = new ValhallaClient({
  baseUrl: 'http://valhalla.localhost',
  clientId: 'my-app',
})

const route = await valhalla.route({
  locations: [
    { lat: 52.5, lon: 13.4 },
    { lat: 52.51, lon: 13.42 },
  ],
  costing: CostingModel.Bicycle,
})
```

## Endpoints

The client exposes wrappers for:

- `route`
- `optimized_route`
- `sources_to_targets`
- `isochrone`
- `trace_route`
- `trace_attributes`
- `locate`
- `height`
- `expansion`
- `status`
- `centroid`
- `tile`

`tile()` returns the raw `Response`; this package does not parse MVT data.

## Transport

Requests default to POST JSON. Per request, pass `{ method: HttpMethod.Get }` to use Valhalla's `?json=` query parameter form.

```ts
import { HttpMethod } from 'valhalla_client'

await valhalla.route(request, { method: HttpMethod.Get })
```

## Development

```sh
npm install
npm run type-check
npm run build
```
