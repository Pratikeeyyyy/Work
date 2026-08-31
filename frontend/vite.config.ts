import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const backend = "http://localhost:8080";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 5173,
    proxy: {
      "/health": backend,
      "/stats": backend,
      "/leads": backend,
      "/clients": backend,
      "/contracts": backend,
      "/settings": backend,
      "/scrape": backend,
    },
  },
});