// Node's libuv threadpool defaults to 4 threads, which rolldown (vite 8's
// bundler) saturates and then deadlocks on: a cold dep-optimize or build sits
// at 0% CPU forever, so tauri gives up after 180s and shows an empty window.
// Set before anything touches the threadpool.
process.env.UV_THREADPOOL_SIZE ??= '16';

import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [tailwindcss(), react()],

  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Explicit IPv4: with plain "localhost", node may bind ::1 only (resolution
    // order shifts with VPN/network state) while tauri probes 127.0.0.1 — the
    // app then waits 180s for a dev server that is already "ready".
    host: host || '127.0.0.1',
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // .DS_Store churn from Finder produces directory-level FSEvents that the
      // watcher re-reports as changes to every sibling source file; the flood
      // can stall startup. graphify-out is regenerated wholesale and is never
      // imported, so watching it is pure noise.
      ignored: ['**/src-tauri/**', '**/graphify-out/**', '**/.DS_Store'],
    },
  },

  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
}));
