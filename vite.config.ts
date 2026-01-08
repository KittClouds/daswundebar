import { defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";
import path from "path";
import { componentTagger } from "lovable-tagger";

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => ({
  server: {
    host: "::",
    port: 8081,
    headers: {
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    },
  },
  plugins: [
    react(),
    mode === "development" && componentTagger(),
    // WASM cache-busting in development
    mode === "development" && {
      name: 'wasm-no-cache',
      configureServer(server) {
        server.middlewares.use((req, res, next) => {
          if (req.url?.includes('.wasm')) {
            res.setHeader('Cache-Control', 'no-store, no-cache, must-revalidate');
            res.setHeader('Pragma', 'no-cache');
          }
          next();
        });
      },
    },
  ].filter(Boolean),
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
      "@kittcore/wasm": path.resolve(__dirname, "./rust/kittcore/pkg"),
    },
  },
  optimizeDeps: {
    exclude: ["@sqliteai/sqlite-wasm", "@kittcore/wasm"],
  },
  worker: {
    format: "es",
  },
  build: {
    target: "esnext",
  },
  test: {
    setupFiles: ['./src/test/vitest.setup.ts'],
    environment: 'node',
    alias: {
      'cozo-lib-wasm': path.resolve(__dirname, './src/test/__mocks__/cozo-lib-wasm.ts'),
    },
    deps: {
      inline: ['cozo-lib-wasm'],
    },
  },
}));
