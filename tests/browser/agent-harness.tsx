import { createRoot } from "react-dom/client";
import { useState } from "react";
import { ReactFlowProvider } from "@xyflow/react";
import "@xyflow/react/dist/style.css";
import "@/index.css";
import { Canvas } from "@/flow/Canvas";
import { AiGenerateDialog } from "@/app/AiGenerateDialog";
import { KeyboardShortcuts } from "@/app/KeyboardShortcuts";
import { api, type GeneratedGraph } from "@/lib/bindings";
import { useGraphStore as G } from "@/store/graph";
import { useAgentStore as A } from "@/store/agent";
import { useAiStore as AI } from "@/store/ai";
import { useRunStore as R } from "@/store/run";
import { useDescriptorStore as D } from "@/store/descriptors";
import catalog from "@/lib/nodeCatalog.generated.json";

D.getState().setDescriptors(catalog as never);
let scenario = "success", jobIndex = 0, cancelled = 0;
const waiting = new Map<string, () => void>();
const waitForJob = async (job: string, ms: number) => new Promise<void>((resolve) => {
  const timer = setTimeout(() => { waiting.delete(job); resolve(); }, ms);
  waiting.set(job, () => { clearTimeout(timer); waiting.delete(job); cancelled++; resolve(); });
});
api.cancelJob = async (job) => waiting.get(job)?.();
api.resetRun = async () => {};
const sample = (): GeneratedGraph => ({ notes: "读取已提供的 Base64 文本，按标准码表解码并显示结果。没有文件读写或网络访问。",
  nodes: [
    { key: "source", descriptorId: "text_input", params: { text: "SGVsbG8=" }, position: { x: 40, y: 60 } },
    { key: "decode", descriptorId: "base64_decode", params: {}, position: { x: 300, y: 60 } },
    { key: "out", descriptorId: "text_output", params: {}, position: { x: 560, y: 60 } },
  ], edges: [{ from: { node: "source", port: "text" }, to: { node: "decode", port: "text" } },
    { from: { node: "decode", port: "text" }, to: { node: "out", port: "text" } }], checks: [],
});
api.generateWorkflow = async (_prompt, emit) => {
  const job = `planning-${++jobIndex}`;
  emit?.({ kind: "jobStarted", job }); emit?.({ kind: "log", level: "info", message: "测试响应：正在生成计划" });
  await waitForJob(job, scenario === "slowPlan" ? 60000 : 250);
  if (scenario === "planError") throw new Error("模型测试响应失败");
  return sample(); // Deliberately resolves after cancellation to test late-result protection.
};
api.explainWorkflow = async (_graph, _prompt, emit) => {
  const job = `explain-${++jobIndex}`; emit?.({ kind: "jobStarted", job });
  await waitForJob(job, scenario === "slowPlan" ? 60000 : 250);
  return { text: "读取文本、解码并显示。" };
};
api.runGraph = async (graph, emit) => {
  const job = `run-${++jobIndex}`;
  emit({ kind: "jobStarted", job });
  emit({ kind: "nodeEntered", node: graph.nodes[0].id });
  await waitForJob(job, scenario === "slowRun" ? 60000 : 200);
  if (scenario === "runError") { emit({ kind: "nodeFailed", node: graph.nodes[1].id, error: "测试响应：无效 Base64" }); throw new Error("无效 Base64"); }
  // Fixtures only: Rust algorithm correctness is covered by ai_workflow tests.
  const result = {};
  for (const node of graph.nodes) {
    emit({ kind: "nodeDone", node: node.id });
    result[node.id] = { [node.descriptorId === "text_output" ? "value" : "text"]:
      { type: "text", value: scenario === "mismatch" ? "Wrong" : node.descriptorId === "text_input" ? node.params.text : "Hello" } };
  }
  return result;
};
function Harness() {
  const nodes = G((s) => s.nodes), edges = G((s) => s.edges), history = R((s) => s.history);
  const [choice, setChoice] = useState("success");
  return <ReactFlowProvider><main className="flex h-screen flex-col bg-background text-foreground">
    <div className="flex flex-wrap items-center gap-4 border-b p-3 text-xs">
      <strong>AI Agent 交互回归 · 模型与执行响应为测试替身</strong>
      <select aria-label="测试场景" value={choice} onChange={(e) => { scenario = e.target.value; setChoice(scenario); }}>
        <option value="success">成功</option><option value="slowPlan">规划延迟</option><option value="slowRun">执行延迟</option>
        <option value="planError">规划失败</option><option value="runError">执行失败</option><option value="mismatch">结果不符</option>
      </select>
      <button onClick={() => AI.getState().setOpen(true)}>AI 助手</button>
      <button onClick={() => G.getState().undo()}>撤销搭建</button>
      <button onClick={() => { G.getState().loadFlow([], []); A.getState().reset(); }}>清空测试画布</button>
      <span aria-label="测试统计">节点 {nodes.length} · 连线 {edges.length} · 已取消 {cancelled} · 运行 {history[0]?.status ?? "无"}</span>
    </div>
    <div className="relative min-h-0 flex-1"><Canvas /></div>
    <AiGenerateDialog /><KeyboardShortcuts />
  </main></ReactFlowProvider>;
}
createRoot(document.getElementById("root")!).render(<Harness />);
