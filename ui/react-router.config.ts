import type { Config } from "@react-router/dev/config";

export default {
  // SPA mode is required for Tauri: it needs a static index.html in
  // build/client/. SSR (the React Router default) only emits assets and
  // a Node server entry, leaving Tauri with a "index.html not found"
  // white window.
  ssr: false,
} satisfies Config;
