/// <reference types="vite/client" />

interface ViteTypeOptions {
  strictImportMetaEnv: unknown
}

interface ImportMetaEnv {
  readonly VITE_TILESERVER_OMT_URL: string
  readonly VITE_API_URL: string
  readonly VITE_VALHALLA_URL: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}