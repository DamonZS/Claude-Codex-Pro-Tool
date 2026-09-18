/* CCP modification: Namespace durable UI preferences away from the Codex host. Upstream attribution: vendor/multica/NOTICE. */
import type { StorageAdapter } from "../types/storage";

/** SSR-safe localStorage. Works in both Next.js (SSR) and Electron (always client). */
export const defaultStorage: StorageAdapter = {
  getItem: (k) =>
    typeof window !== "undefined" ? localStorage.getItem("ccp.workflow." + k) : null,
  setItem: (k, v) => {
    if (typeof window !== "undefined") localStorage.setItem("ccp.workflow." + k, v);
  },
  removeItem: (k) => {
    if (typeof window !== "undefined") localStorage.removeItem("ccp.workflow." + k);
  },
};
