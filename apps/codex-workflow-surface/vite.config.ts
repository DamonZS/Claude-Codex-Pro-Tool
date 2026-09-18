import { fileURLToPath, URL } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const fromRoot = (path: string) => fileURLToPath(new URL(path, import.meta.url));
const multicaPackage = (name: string) => fromRoot(`./vendor/multica/packages/${name}`);

export default defineConfig({
  plugins: [react(), tailwindcss()],
  define: { "process.env.NODE_ENV": JSON.stringify("production") },
  resolve: {
    alias: {
      "@multica/ui/i18n-types": multicaPackage("ui/types/i18next.ts"),
      "@multica/core": multicaPackage("core"),
      "@multica/ui": multicaPackage("ui"),
      "@multica/views": multicaPackage("views"),
      "@ccp/workflow-host": fromRoot("./src/upstream-host.ts"),
    },
  },
  build: {
    emptyOutDir: true,
    lib: {
      entry: fromRoot("./src/main.tsx"),
      formats: ["iife"],
      name: "CodexWorkflowSurface",
      fileName: () => "codex-workflow-surface.js",
    },
    rollupOptions: {
      output: {
        inlineDynamicImports: true,
        assetFileNames: "codex-workflow-surface.[ext]",
      },
    },
  },
});
