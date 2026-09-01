import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const backend = "http://localhost:8080";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 5173,
    proxy: {
      "/health": backend,
      "/login": backend,
      "/register": backend,
      "/auth": backend,
      "/stats": backend,
      "/leads": backend,
      "/clients": backend,
      "/contracts": backend,
      "/applications": backend,
      "/profile": backend,
      "/linkedin": backend,
      "/settings": backend,
      "/scrape": backend,
    },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: "./src/test/setup.ts",
    css: false,
  },
});