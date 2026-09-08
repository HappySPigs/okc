/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

// The backend serves `frontend/dist` as the SPA and proxies same-origin `/api`
// + `/u`. In dev we proxy those to the FastAPI process on :8000.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: { "@": path.resolve(__dirname, "src") },
  },
  server: {
    proxy: {
      "/api": { target: "http://localhost:8000", changeOrigin: true },
      // Trailing slash so this matches the /u/{token} API without swallowing the
      // /upload/* SPA routes (contributor portal).
      "/u/": { target: "http://localhost:8000", changeOrigin: true },
    },
  },
  build: { outDir: "dist" },
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    css: false,
  },
});
