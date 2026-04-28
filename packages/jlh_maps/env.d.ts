/// <reference types="vite/client" />

interface ViteTypeOptions {
    strictImportMetaEnv: unknown
}

interface ImportMetaEnv {
    readonly VITE_OTM_TILESERVER_TILEJSON_URL: string
}

interface ImportMeta {
    readonly env: ImportMetaEnv
}