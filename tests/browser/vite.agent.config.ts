import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath } from "node:url";

// Separate, local-only fixture server. Production never enables this transport.
export default defineConfig({
  plugins: [{ name: "agent-fixture-transport", enforce: "pre", resolveId(id) {
    if (id === "@/lib/devMocks" || id === "./devMocks") return "\0agent-fixture";
  }, load(id) { if (id === "\0agent-fixture" || id.replaceAll("\\", "/").endsWith("/src/lib/devMocks.ts")) return "export const inTauri=true;"; } }, react(), tailwindcss()],
  resolve: { alias: { "@": fileURLToPath(new URL("../../src", import.meta.url)) } },
  server: { host: "127.0.0.1", port: 1422, strictPort: true,
    watch: { ignored: ["**/target/**", "**/dist/**", "**/src-tauri/**"] } },
});
