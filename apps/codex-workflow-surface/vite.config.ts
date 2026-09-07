import { fileURLToPath, URL } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const fromRoot = (path: string) => fileURLToPath(new URL(path, import.meta.url));
const multicaRoot = process.env.MULTICA_SOURCE_ROOT || "D:/Project/multica";
const multicaPackage = (name: string) => fileURLToPath(new URL(`file:///${multicaRoot.replace(/\\/g, "/").replace(/^\/+/, "")}/packages/${name}`));

export default defineConfig({
  plugins: [react(), tailwindcss()],
  // The copied packages retain their upstream per-package tsconfig files. They
  // extend the upstream monorepo preset, which intentionally is not a runtime
  // dependency of this derived bundle. Force Vite's transform to use this
  // package's compatible compiler settings instead of walking that monorepo.
  esbuild: {
    tsconfigRaw: {
      compilerOptions: {
        jsx: "react-jsx",
        target: "es2023",
        useDefineForClassFields: true,
      },
    },
  },
  resolve: {
    alias: {
      "@multica/core": multicaPackage("core"),
      "@multica/ui": multicaPackage("ui"),
      "@multica/views": multicaPackage("views"),
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
        assetFileNames: "codex-workflow-surface.[ext]",
      },
    },
  },
});
