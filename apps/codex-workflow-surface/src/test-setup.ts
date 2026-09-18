import { vi } from "vitest";

if (typeof window !== "undefined") {
// The embedded host is Chromium; jsdom otherwise advertises Apple's vendor.
Object.defineProperty(navigator, "vendor", { value: "Google Inc.", configurable: true });
Object.defineProperty(navigator, "userAgent", { value: "Mozilla/5.0 Chrome/130.0.0.0 Safari/537.36", configurable: true });
Object.defineProperty(HTMLCanvasElement.prototype, "getContext", { value: () => null, configurable: true });
// jsdom has no native top layer. NWSAPI's native-state fallback recursively calls
// Element.matches for these selectors when Floating UI measures a nested popup.
const matches = Element.prototype.matches;
Element.prototype.matches = function (selector: string) {
  if (selector === ":fullscreen") return this.ownerDocument.fullscreenElement === this;
  if (selector === ":modal" || selector === ":popover-open") return false;
  return matches.call(this, selector);
};
if (!window.PointerEvent) {
  class PointerEvent extends MouseEvent {
    readonly pointerId: number;
    readonly pointerType: string;
    readonly isPrimary: boolean;
    constructor(type: string, init: PointerEventInit = {}) {
      super(type, init);
      this.pointerId = init.pointerId ?? 0;
      this.pointerType = init.pointerType ?? "mouse";
      this.isPrimary = init.isPrimary ?? true;
    }
  }
  window.PointerEvent = PointerEvent as typeof window.PointerEvent;
}
// jsdom has no editing commands; ProseMirror probes this for shadow selection.
Object.defineProperty(document, "execCommand", { value: () => false, configurable: true });
Object.defineProperty(window, "matchMedia", { writable: true, value: vi.fn().mockImplementation((query: string) => ({
  matches: false, media: query, onchange: null,
  addListener: vi.fn(), removeListener: vi.fn(), addEventListener: vi.fn(), removeEventListener: vi.fn(), dispatchEvent: vi.fn(),
})) });
class Observer {
  observe() {}
  unobserve() {}
  disconnect() {}
}
vi.stubGlobal("ResizeObserver", Observer);
vi.stubGlobal("IntersectionObserver", Observer);
Object.defineProperty(HTMLElement.prototype, "scrollIntoView", { value: vi.fn(), configurable: true });
Object.defineProperty(HTMLElement.prototype, "scrollTo", { value: vi.fn(), configurable: true });
Object.defineProperty(HTMLElement.prototype, "hasPointerCapture", { value: () => false, configurable: true });
Object.defineProperty(HTMLElement.prototype, "setPointerCapture", { value: vi.fn(), configurable: true });
Object.defineProperty(HTMLElement.prototype, "releasePointerCapture", { value: vi.fn(), configurable: true });
Object.defineProperty(window, "IS_REACT_ACT_ENVIRONMENT", { value: true, configurable: true, writable: true });
}
