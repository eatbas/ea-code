/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Show pipeline debug log panel. Set to "true" in .env for local dev. */
  readonly VITE_MAESTRO_DEV?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
