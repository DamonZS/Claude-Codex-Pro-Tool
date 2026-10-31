import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";
import { defineConfig } from "vitest/config";

export default defineConfig({
  base: "./",
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
  },
  // The interface is a set of screens with behaviour — a frame that must appear
  // only when the probe says so, a checklist that must name the missing step, a
  // toast that must carry the backend's own sentence. `npm run check` proves the
  // types and `vite:build` proves it bundles; neither proves any of that, so the
  // suite drives the screens in jsdom the way a person drives them.
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    restoreMocks: true,
    setupFiles: ["./src/test/setup.ts"],
  },
});
