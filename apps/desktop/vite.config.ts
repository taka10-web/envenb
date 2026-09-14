import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

// https://v2.tauri.app/start/frontend/vite/
// Dev only: the webview posts uncaught errors here so they show up in the Vite log.
const devErrorLog = () => ({
  name: "envenb-dev-error-log",
  configureServer(server: { middlewares: { use: (fn: (req: any, res: any, next: () => void) => void) => void } }) {
    server.middlewares.use((req, res, next) => {
      if (req.url === "/__envenb_log" && req.method === "POST") {
        let body = "";
        req.on("data", (c: Buffer) => (body += c));
        req.on("end", () => {
          console.error(`[webview] ${body}`);
          res.statusCode = 204;
          res.end();
        });
        return;
      }
      next();
    });
  },
});

export default defineConfig({
  plugins: [react(), tailwindcss(), devErrorLog()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
