import { JSDOM } from "jsdom";

const dom = new JSDOM("<!doctype html><html><body></body></html>", {
  url: "http://localhost/",
});

const { window } = dom;

function copyWindowProperties(): void {
  for (const key of Object.getOwnPropertyNames(window)) {
    if (key in globalThis) {
      continue;
    }

    Object.defineProperty(globalThis, key, {
      configurable: true,
      enumerable: true,
      get: () => window[key as keyof Window & keyof typeof globalThis],
    });
  }
}

Object.defineProperty(globalThis, "window", {
  configurable: true,
  value: window,
});
Object.defineProperty(globalThis, "document", {
  configurable: true,
  value: window.document,
});
Object.defineProperty(globalThis, "navigator", {
  configurable: true,
  value: window.navigator,
});
Object.defineProperty(globalThis, "localStorage", {
  configurable: true,
  value: window.localStorage,
});
Object.defineProperty(globalThis, "sessionStorage", {
  configurable: true,
  value: window.sessionStorage,
});

copyWindowProperties();

const requestAnimationFrame = (callback: FrameRequestCallback): number =>
  window.setTimeout(() => callback(Date.now()), 0);
const cancelAnimationFrame = (handle: number): void => {
  window.clearTimeout(handle);
};

Object.defineProperty(window, "requestAnimationFrame", {
  configurable: true,
  value: requestAnimationFrame,
});
Object.defineProperty(window, "cancelAnimationFrame", {
  configurable: true,
  value: cancelAnimationFrame,
});
Object.defineProperty(globalThis, "requestAnimationFrame", {
  configurable: true,
  value: requestAnimationFrame,
});
Object.defineProperty(globalThis, "cancelAnimationFrame", {
  configurable: true,
  value: cancelAnimationFrame,
});
Object.defineProperty(globalThis, "IS_REACT_ACT_ENVIRONMENT", {
  configurable: true,
  writable: true,
  value: true,
});

if (!("ResizeObserver" in globalThis)) {
  class ResizeObserver {
    public disconnect(): void {}
    public observe(): void {}
    public unobserve(): void {}
  }

  Object.defineProperty(globalThis, "ResizeObserver", {
    configurable: true,
    value: ResizeObserver,
  });
}
