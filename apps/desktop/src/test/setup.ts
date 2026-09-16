import "@testing-library/jest-dom/vitest";
import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";

// jsdom has no matchMedia; ThemeProvider and the i18n helpers expect one.
if (typeof window !== "undefined" && typeof window.matchMedia !== "function") {
  window.matchMedia = (query: string): MediaQueryList => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  });
}

afterEach(() => {
  cleanup();
  try {
    window.localStorage.clear();
  } catch {
    /* ignore */
  }
});

// `findBy*` and `waitFor` default to 1s. That is tight on a CI runner where a
// render plus a mocked query can take longer, so give them room: a real
// failure still fails, just later.
import { configure } from "@testing-library/dom";
configure({ asyncUtilTimeout: 10_000 });
