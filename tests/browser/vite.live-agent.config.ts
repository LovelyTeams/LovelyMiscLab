import { defineConfig } from "vite";
import base from "./vite.agent.config";
import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import { resolve } from "node:path";
import { appendFileSync } from "node:fs";

// Opt-in only. Loopback, strict Origin + custom header, and a Rust allowlist.
// No model configuration or credentials are exposed to the browser.
export default defineConfig({ ...base, server: { ...base.server, port: 1423, hmr: false }, plugins: [
  ...base.plugins!, { name: "real-rust-agent-transport", configureServer(server) {
    const binary = process.env.MISCLAB_LIVE_TEST_BIN;
    if (!binary) throw new Error("Set MISCLAB_LIVE_TEST_BIN to the compiled test executable");
    const child = spawn(resolve(binary), ["live_agent_bridge", "--ignored", "--nocapture", "--test-threads=1"], {
      env: { ...process.env, MISCLAB_LIVE_AI: "1" }, windowsHide: true, stdio: ["pipe", "pipe", "pipe"],
    });
    const pending = new Map<string, import("node:http").ServerResponse>();
    const operations = new Map<string, string>();
    let stopped = false;
    const stop = () => {
      stopped = true;
      for (const [id, res] of pending) res.end(JSON.stringify({ id, kind: "error", value: "Live test process stopped" }) + "\n");
      pending.clear();
    };
    const lines = createInterface({ input: child.stdout });
    lines.on("line", (line) => {
      if (!line.startsWith("AI_LIVE:")) return;
      const message = JSON.parse(line.slice(8));
      const response = pending.get(message.id);
      if (!response) return;
      // Optional local test evidence: model plans only, never saved credentials
      // or original prompts. Disabled unless the tester explicitly opts in.
      if (process.env.MISCLAB_LIVE_PLAN_AUDIT && message.kind === "result" && ["generate", "decide"].includes(operations.get(message.id) ?? ""))
        appendFileSync(resolve(".git/live-agent-plan-audit.jsonl"), JSON.stringify({ time: new Date().toISOString(), op: operations.get(message.id), ...message }) + "\n");
      response.write(JSON.stringify(message) + "\n");
      if (message.kind !== "event") { response.end(); pending.delete(message.id); operations.delete(message.id); }
    });
    child.on("exit", stop);
    child.on("error", stop);
    child.stdin.on("error", stop);
    server.httpServer?.once("close", () => { child.stdin.end(); child.kill(); });
    let sequence = 0;
    server.middlewares.use("/__live_agent", async (req, res) => {
      if (req.method !== "POST" || req.headers.origin !== "http://127.0.0.1:1423" || req.headers["x-agent-test"] !== "live") {
        res.statusCode = 403; res.end(); return;
      }
      if (stopped) { res.statusCode = 503; res.end("Live test process stopped"); return; }
      const chunks: Buffer[] = []; let size = 0;
      for await (const chunk of req) { size += chunk.length; if (size > 2_000_000) { res.statusCode = 413; res.end(); return; } chunks.push(chunk); }
      try {
        const request = JSON.parse(Buffer.concat(chunks).toString());
        const id = String(++sequence); pending.set(id, res);
        operations.set(id, request.op);
        res.setHeader("Content-Type", "application/x-ndjson"); res.setHeader("Cache-Control", "no-store");
        child.stdin.write(JSON.stringify({ ...request, id }) + "\n");
      } catch { res.statusCode = 400; res.end(); }
    });
  } },
] });
