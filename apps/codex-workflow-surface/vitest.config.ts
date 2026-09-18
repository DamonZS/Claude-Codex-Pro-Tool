import { fileURLToPath } from "node:url";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

const local = (path: string) => fileURLToPath(new URL(path, import.meta.url));
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@multica/ui/i18n-types": local("./vendor/multica/packages/ui/types/i18next.ts"),
      "@multica/core": local("./vendor/multica/packages/core"),
      "@multica/ui": local("./vendor/multica/packages/ui"),
      "@multica/views": local("./vendor/multica/packages/views"),
      "@ccp/workflow-host": local("./src/upstream-host.ts"),
    },
  },
  test: {
    include: ["src/**/*.test.{ts,tsx}"],
    environment: "jsdom",
    setupFiles: ["./src/test-setup.ts"],
    testTimeout: 20000,
  },
});
