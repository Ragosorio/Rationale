/// <reference types="vitest/config" />
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// La UI se sirve embebida en el binario (`build.rs` lee `ui/dist`). En
// desarrollo, `npm run dev` delega la API y el stream al backend real de
// `rationale ui`, que solo acepta un Host de loopback con su propio puerto:
// por eso `changeOrigin`.
const backend = process.env.RATIONALE_UI_BACKEND ?? "http://127.0.0.1:9748";

export default defineConfig({
  plugins: [react()],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    sourcemap: false,
    chunkSizeWarningLimit: 1800,
  },
  server: {
    proxy: {
      "/api": { target: backend, changeOrigin: true },
      "/events": { target: backend, changeOrigin: true },
    },
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
