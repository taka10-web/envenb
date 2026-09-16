import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
    // A CI runner is slower than a laptop, and these tests wait on React
    // Query resolving a mocked fetch. The default 1s is enough locally and
    // not on a shared runner, which shows up as a flake rather than a bug.
    testTimeout: 15_000,
  },
});
