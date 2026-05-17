import { ValhallaClient } from 'valhalla_client'
import { VALHALLA_URL } from '@/external/endpoints.ts'

export const valhallaClient = new ValhallaClient({
  baseUrl: VALHALLA_URL,
  clientId: 'jlh-maps',
  fetch: (...args) => fetch(...args),
})
