import { createRoot } from "react-dom/client";
import { ReactFlowProvider } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import "@/index.css";
import { Canvas } from "@/flow/Canvas";
import { AiGenerateDialog } from "@/app/AiGenerateDialog";
import { KeyboardShortcuts } from "@/app/KeyboardShortcuts";
import { buildProject } from "@/lib/project";
import { api } from "@/lib/bindings";
import type { ArtifactMetadata, PortValue, ProgressMsg } from "@/lib/types";
import { useGraphStore as G } from "@/store/graph";
import { useAgentStore as A } from "@/store/agent";
import { useAiStore as AI } from "@/store/ai";
import { useRunStore as R } from "@/store/run";
import { useDescriptorStore as D } from "@/store/descriptors";
import catalog from "@/lib/nodeCatalog.generated.json";
D.getState().setDescriptors(catalog as never);
async function call<T>(body: object, emit?: (event: ProgressMsg) => void): Promise<T> {
  const response = await fetch("/__live_agent", { method: "POST", headers: { "Content-Type": "application/json", "X-Agent-Test": "live" }, body: JSON.stringify(body) });
  if (!response.ok || !response.body) throw new Error(`测试传输失败：${response.status}`);
  const reader = response.body.getReader(), decoder = new TextDecoder(); let buffer = "";
  try {
    for (;;) {
      const { done, value } = await reader.read();
      if (done) throw new Error("测试传输提前结束");
      buffer += decoder.decode(value, { stream: true });
      let end: number;
      while ((end = buffer.indexOf("\n")) >= 0) {
        const line = buffer.slice(0, end); buffer = buffer.slice(end + 1); if (!line) continue;
        const item = JSON.parse(line);
        if (item.kind === "event") emit?.(item.value);
        else if (item.kind === "error") throw new Error(item.value);
        else return item.value as T;
      }
    }
  } finally { reader.releaseLock(); }
}
api.decideSolverStep = (prompt, emit) => call({ op: "decide", prompt }, emit);
api.generateWorkflow = (prompt, emit, catalogIds) => call({ op: "generate", prompt, catalogIds }, emit);
const artifactMetadata = new Map<string, ArtifactMetadata>();
api.runGraph = async (graph, emit) => {
  const outputs = await call<Record<string, Record<string, PortValue>>>({ op: "run", graph }, emit);
  // Reuse metadata returned by the real Rust artifact_store. This browser
  // harness has no Tauri invoke bridge and must not call one for its preview.
  for (const values of Object.values(outputs)) {
    const metadata = values.metadata;
    if (metadata?.type !== "json" || !metadata.value || typeof metadata.value !== "object") continue;
    const meta = metadata.value as unknown as ArtifactMetadata;
    if (typeof meta.id === "string" && typeof meta.size === "number" && typeof meta.sha256 === "string") artifactMetadata.set(meta.id, meta);
  }
  return outputs;
};
api.artifactMetadata = async (id) => {
  const metadata = artifactMetadata.get(id);
  if (!metadata) throw new Error("本次测试未返回该产物的元数据");
  return metadata;
};
api.cancelJob = (job) => call({ op: "cancel", job });
api.resetRun = async () => {}; // Isolated executor has no cross-run cache.
function LiveHarness() {
  const nodes = G((s) => s.nodes), edges = G((s) => s.edges), history = R((s) => s.history);
  return <ReactFlowProvider><main className="flex h-screen flex-col bg-background text-foreground">
    <div className="flex items-center gap-4 border-b p-3 text-xs">
      <strong>真实模型联调 · 已保存模型 + Rust 执行器</strong>
      <button onClick={() => AI.getState().setOpen(true)}>AI 助手</button>
      <button onClick={() => G.getState().undo()}>撤销搭建</button>
      <button onClick={() => { G.getState().loadFlow([], []); A.getState().reset(); }}>清空测试画布</button>
      <span>节点 {nodes.length} · 连线 {edges.length} · 运行 {history[0]?.status ?? "无"}</span>
      <details className="relative"><summary className="cursor-pointer">导出测试流程</summary>
        <textarea aria-label="测试流程 JSON" readOnly className="absolute right-0 z-50 h-80 w-96 border bg-background p-2 font-mono"
          value={JSON.stringify({ ...buildProject(), name: "AI 复杂流程联调" }, null, 2)} />
      </details>
    </div>
    <div className="relative min-h-0 flex-1"><Canvas /></div>
    <AiGenerateDialog /><KeyboardShortcuts />
  </main></ReactFlowProvider>;
}
const root = createRoot(document.getElementById("root")!);
root.render(<LiveHarness />);
if (import.meta.hot) import.meta.hot.dispose(() => root.unmount());
