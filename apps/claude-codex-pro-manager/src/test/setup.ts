import { cleanup } from "@testing-library/react";
import { afterEach, vi } from "vitest";

// jsdom ships no layout engine and no media queries, so any screen that asks
// whether the desktop is dark or how wide the window is would throw instead of
// rendering. A stub that answers "no" keeps those screens renderable; the suite
// is about behaviour, not about what the OS theme happens to be.
if (typeof window.matchMedia !== "function") {
  window.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: () => {},
    removeEventListener: () => {},
    addListener: () => {},
    removeListener: () => {},
    dispatchEvent: () => false,
  })) as unknown as typeof window.matchMedia;
}

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});