export const TILESERVER_URL = new URL(import.meta.env.VITE_TILESERVER_OMT_URL)
export const API_URL = new URL(import.meta.env.VITE_API_URL)

export const TILESERVER_OMT_DEFAULT_STYLE_TILEJSON_URL = new URL(
  'styles/omt_default/style.json',
  TILESERVER_URL,
)
