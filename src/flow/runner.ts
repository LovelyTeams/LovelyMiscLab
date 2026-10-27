import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import type { NodeDescriptor, ParamWidget, PortValue, ProgressMsg, SerializedGraph } from "@/lib/types";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";
import { useProjectStore } from "@/store/project";
import { useRunStore } from "@/store/run";

// Module-level run coordination (single in-flight run; latest state coalesced).
interface ActiveRun {
  standalone?: boolean;
  job: string | null;
  cancelled: boolean;
  clear: boolean;
  revision: number;
  failure?: string;
  finished: Promise<void>;
  resolve: () => void;
}
let active: ActiveRun | null = null;
// An Agent keeps ownership while planning between executions. Manual runs must
// not slip into those gaps; stop also cancels the planning request.
let foreground: { owner: symbol; cancel: () => void } | null = null;
export function acquireForeground(cancel: () => void): { owner: symbol; release: () => void } {
  if (active || foreground) throw new Error("已有任务正在运行");
  const lease = { owner: Symbol("agent"), cancel };
  foreground = lease;
  useRunStore.getState().setRunning(true);
  return { owner: lease.owner, release: () => {
    if (foreground !== lease) return;
    foreground = null;
    useRunStore.getState().setRunning(!!active);
  } };
}
let pending = false;
export interface RunOutcome { status: "success" | "error" | "cancelled"; error?: string; historyId?: string }

function errorText(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object" && "message" in error) return String(error.message);
  return String(error);
}

function now() {
  return new Date().toLocaleTimeString();
}

/** Coerce a connected port value into the JS type a param widget expects (mirrors
 * the Rust executor), so single-node execution honors param connections too. */
function coerceParam(v: PortValue, widget: ParamWidget): unknown {
  if (widget.kind === "number" || widget.kind === "slider") {
    if (v.type === "number") return v.value;
    if (v.type === "bool") return v.value ? 1 : 0;
    if (v.type === "text") {
      const n = Number(v.value.trim());
      return v.value.trim() && Number.isFinite(n) ? n : null;
    }
    return 0;
  }
  if (widget.kind === "toggle") {
    if (v.type === "bool") return v.value;
    if (v.type === "number") return v.value !== 0;
    if (v.type === "text")
      return ["true", "1", "yes", "on", "是"].includes(v.value.trim().toLowerCase());
    return false;
  }
  if (v.type === "text") return v.value;
  if (v.type === "number") return String(v.value);
  if (v.type === "bool") return String(v.value);
  if (v.type === "stringList") return v.value.join("\n");
  return "";
}

/** Serialize the current graph, excluding disabled nodes (and their edges). */
export function buildGraph(onlyIds?: Set<string>): SerializedGraph {
  const g = useGraphStore.getState();
  const enabled = g.nodes.filter((n) => !n.data.disabled && (!onlyIds || onlyIds.has(n.id)));
  const ids = new Set(enabled.map((n) => n.id));
  return {
    nodes: enabled.map((n) => ({
      id: n.id,
      descriptorId: n.data.descriptorId,
      params: n.data.params,
      position: [n.position.x, n.position.y],
    })),
    edges: g.edges
      .filter(
        (e) =>
          e.sourceHandle && e.targetHandle && ids.has(e.source) && ids.has(e.target)
      )
      .map((e) => ({
        from: { node: e.source, port: e.sourceHandle as string },
        to: { node: e.target, port: e.targetHandle as string },
      })),
  };
}

function handleEvent(m: ProgressMsg, run: ActiveRun) {
  if (active !== run) return;
  if (m.kind === "jobStarted") {
    run.job = m.job;
    if (run.cancelled) void api.cancelJob(m.job).catch(() => {});
  }
  useRunStore.getState().recordProgress(m);
  if (run.cancelled || run.revision !== useGraphStore.getState().runRevision) return;
  const s = useGraphStore.getState();
  switch (m.kind) {
    case "jobStarted":
      break;
    case "nodeEntered":
      s.updateRuntime(m.node, {
        status: "running",
        progress: 0,
        error: undefined,
        logs: [{ time: now(), level: "info", message: "开始执行" }],
      });
      break;
    case "nodeProgress":
      s.updateRuntime(m.node, { progress: m.pct });
      break;
    case "nodeDone":
      s.updateRuntime(m.node, { status: "done", progress: 1 });
      s.appendLog(m.node, { time: now(), level: "success", message: "执行成功" });
      break;
    case "nodeFailed":
      run.failure = m.error;
      s.updateRuntime(m.node, { status: "error", error: m.error });
      s.appendLog(m.node, { time: now(), level: "error", message: m.error });
      break;
    case "jobFailed":
      run.failure = m.error;
      useRunStore.getState().setLastError(m.error);
      break;
    case "log":
      if (m.node) s.appendLog(m.node, { time: now(), level: m.level, message: m.message });
      break;
    default:
      break;
  }
}

/** Truncate an output value so run history stays small enough for localStorage. */
export function sanitizeValue(v: PortValue): PortValue {
  const cap = (s: string, max: number) => (s.length > max ? `${s.slice(0, max)}…` : s);
  switch (v.type) {
    case "text":
      return { type: "text", value: cap(v.value, 20000) };
    case "bytes":
      return { type: "text", value: `[${v.value.length} 字节；运行记录只保存摘要，请从运行结果导出完整文件]` };
    case "image":
      // Never persist inline media; artifacts keep their UUID reference below.
      return { type: "text", value: "[图片；运行记录只保存摘要]" };
    case "stringList":
      return { type: "stringList", value: v.value.slice(0, 200).map((s) => cap(s, 2000)) };
    case "candidates":
      return { type: "candidates", value: v.value.slice(0, 50).map((c) => ({ ...c, text: cap(c.text, 2000), note: c.note && cap(c.note, 1000) })) };
    case "json":
    case "fingerprint": {
      const s = JSON.stringify(v.value);
      return s && s.length > 8000 ? { type: "text", value: cap(s, 8000) } : v;
    }
    default:
      return v;
  }
}

export function sanitizeOutputs(
  outputs?: Record<string, PortValue>
): Record<string, PortValue> | undefined {
  if (!outputs) return undefined;
  const out: Record<string, PortValue> = {};
  for (const [k, v] of Object.entries(outputs)) out[k] = sanitizeValue(v);
  return Object.keys(out).length ? out : undefined;
}

function historyNodes(ids?: Set<string>) {
  const byId = useDescriptorStore.getState().byId;
  return useGraphStore.getState().nodes.filter((n) => !ids || ids.has(n.id)).map((n) => ({
    id: n.id,
    label: n.data.label || byId[n.data.descriptorId]?.displayName || n.data.descriptorId,
    descriptorId: n.data.descriptorId,
    status: n.data.status,
    error: n.data.error,
    outputs: sanitizeOutputs(n.data.outputs),
  }));
}

function startHistory(scope: "graph" | "node" | "debug", title: string, graph: SerializedGraph) {
  return useRunStore.getState().startHistory({
    scope,
    title,
    projectName: useProjectStore.getState().name,
    nodeCount: graph.nodes.length,
    edgeCount: graph.edges.length,
    nodes: historyNodes(new Set(graph.nodes.map((n) => n.id))),
  });
}

function finishHistory(entryId: string, elapsed: number, graph: SerializedGraph, error?: unknown) {
  const text = error ? String(error) : "";
  const ids = new Set(graph.nodes.map((n) => n.id));
  const status = text
    ? text.toLowerCase().includes("cancel")
      ? "cancelled"
      : "error"
    : useGraphStore.getState().nodes.some((n) => ids.has(n.id) && n.data.status === "error")
      ? "error"
      : "success";
  useRunStore.getState().finishHistory(entryId, {
    elapsed,
    status,
    error: text || undefined,
    nodes: historyNodes(ids),
  });
}

async function runOperation(
  graph: SerializedGraph,
  scope: "graph" | "debug" | "node",
  title: string,
  operation: (onEvent: (event: ProgressMsg) => void) => Promise<Record<string, Record<string, PortValue>>>,
  owner?: symbol,
): Promise<RunOutcome> {
  if (!inTauri) {
    const error = "浏览器预览无法执行节点，请在桌面应用中运行";
    useRunStore.getState().setLastError(error);
    return { status: "error", error };
  }
  if (active || (foreground && foreground.owner !== owner)) return { status: "error", error: "已有任务正在运行" };
  if (!graph.nodes.length) return { status: "error", error: "没有可执行的节点" };
  let resolve!: () => void;
  const run: ActiveRun = {
    job: null, cancelled: false, clear: false, revision: useGraphStore.getState().runRevision,
    finished: new Promise<void>((done) => { resolve = done; }), resolve: () => resolve(),
  };
  active = run;
  const ids = new Set(graph.nodes.map((n) => n.id));
  for (const id of ids) useGraphStore.getState().updateRuntime(id, {
    status: "idle", progress: 0, outputs: undefined, error: undefined, logs: [],
  });
  useRunStore.getState().setRunning(true);
  useRunStore.getState().setLastError(null);
  const started = Date.now();
  const entry = startHistory(scope, title, graph);
  let outcome: RunOutcome = { status: "success" };
  try {
    const outputs = await operation((event) => handleEvent(event, run));
    if (!run.cancelled && run.revision === useGraphStore.getState().runRevision) {
      for (const [id, values] of Object.entries(outputs))
        useGraphStore.getState().updateRuntime(id, { outputs: values });
      const failed = useGraphStore.getState().nodes.find((n) => ids.has(n.id) && n.data.status === "error");
      if (run.failure || failed) outcome = { status: "error", error: run.failure || failed?.data.error || "节点执行失败" };
    } else if (!run.cancelled) {
      outcome = { status: "cancelled", error: "画布已修改，本次旧结果已丢弃" };
    }
  } catch (error) {
    outcome = { status: run.cancelled ? "cancelled" : "error", error: errorText(error) };
  } finally {
    if (run.cancelled) outcome = { status: "cancelled" };
    const elapsed = Date.now() - started;
    if (outcome.status === "error") useRunStore.getState().setLastError(outcome.error ?? "执行失败");
    finishHistory(entry, elapsed, graph, outcome.status === "cancelled" ? "cancelled" : outcome.error);
    for (const node of useGraphStore.getState().nodes) {
      if (ids.has(node.id) && node.data.status === "running")
        useGraphStore.getState().updateRuntime(node.id, { status: "idle", progress: 0 });
    }
    if (run.clear) {
      try { await api.resetRun(); } catch { /* backend may have disconnected */ }
      useGraphStore.getState().resetRuntime(ids);
    }
    active = null;
    useRunStore.getState().setRunning(!!foreground);
    useRunStore.getState().setElapsed(elapsed);
    run.resolve();
    const rerun = pending && !run.cancelled && useRunStore.getState().mode === "live";
    pending = false;
    if (rerun) void executeGraph();
  }
  return { ...outcome, historyId: entry };
}

function runSerializedGraph(graph: SerializedGraph, scope: "graph" | "debug", title: string, owner?: symbol) {
  return runOperation(graph, scope, title, (onEvent) => api.runGraph(graph, onEvent), owner);
}

export async function executeGraph(): Promise<RunOutcome> {
  if (foreground) return { status: "error", error: "已有任务正在运行" };
  if (active) {
    pending = !active.standalone && useRunStore.getState().mode === "live";
    return { status: "error", error: "已有任务正在运行" };
  }
  return runSerializedGraph(buildGraph(), "graph", "整图运行");
}

export function executeNodeSet(ids: Set<string>, title: string, owner?: symbol): Promise<RunOutcome> {
  return runSerializedGraph(buildGraph(ids), "debug", title, owner);
}

export function executeToNode(nodeId: string): Promise<RunOutcome> {
  const g = useGraphStore.getState();
  const ids = new Set<string>();
  const pendingIds = [nodeId];
  while (pendingIds.length) {
    const id = pendingIds.pop()!;
    if (ids.has(id)) continue;
    ids.add(id);
    for (const edge of g.edges) if (edge.target === id) pendingIds.push(edge.source);
  }
  return executeNodeSet(ids, `运行到：${g.nodes.find((n) => n.id === nodeId)?.data.label || nodeId}`);
}

export async function runSingleNode(nodeId: string): Promise<RunOutcome> {
  if (active || foreground) return { status: "error", error: "已有任务正在运行" };
  const g = useGraphStore.getState();
  const node = g.nodes.find((n) => n.id === nodeId);
  if (!node || node.data.disabled) return { status: "error", error: "节点不存在或已禁用" };
  const descriptor = useDescriptorStore.getState().byId[node.data.descriptorId];
  const inputs: Record<string, PortValue> = {};
  const params = { ...node.data.params };
  const missingConnected: string[] = [];
  for (const edge of g.edges) {
    if (edge.target !== nodeId || !edge.sourceHandle || !edge.targetHandle) continue;
    const source = g.nodes.find((n) => n.id === edge.source);
    const value = source?.data.disabled ? undefined : source?.data.outputs?.[edge.sourceHandle];
    if (!value) { missingConnected.push(edge.targetHandle); continue; }
    if (descriptor?.inputs.some((p) => p.name === edge.targetHandle)) inputs[edge.targetHandle] = value;
    else {
      const spec = descriptor?.params.find((p) => p.name === edge.targetHandle);
      if (spec) params[edge.targetHandle] = coerceParam(value, spec.widget);
    }
  }
  const missing = descriptor?.inputs.filter((p) => p.required && !(p.name in inputs)) ?? [];
  if (missing.length || missingConnected.length) {
    const error = `缺少输入：${[...new Set([...missing.map((p) => p.label), ...missingConnected])].join("、")}，请使用「运行到此节点」执行上游`;
    g.updateRuntime(nodeId, { status: "error", error, outputs: undefined });
    return { status: "error", error };
  }
  return runOperation(buildGraph(new Set([nodeId])), "node", `单节点运行：${node.data.label}`, async (onEvent) => {
    const outputs = await api.runNodeStreamed(node.data.descriptorId, nodeId, inputs, params, onEvent);
    return { [nodeId]: outputs };
  });
}

async function cancelActive(clear: boolean) {
  pending = false;
  const run = active;
  if (!run) {
    if (clear && !foreground) {
      try { await api.resetRun(); } catch { /* browser preview */ }
      useGraphStore.getState().resetRuntime();
    }
    return;
  }
  run.cancelled = true;
  run.clear ||= clear;
  if (run.job) {
    try { await api.cancelJob(run.job); } catch { /* still discard any late results */ }
  }
  // A stop before jobStarted is handled when that event arrives.
  await run.finished;
}

export async function pauseRun() {
  useRunStore.getState().setMode("paused");
  foreground?.cancel();
  await cancelActive(false);
}

export async function stopRun() {
  useRunStore.getState().setMode("idle");
  foreground?.cancel();
  await cancelActive(true);
}

/** Same foreground lock/cancellation lifecycle as the canvas, with local outputs.
 * The abort signal belongs to the dialog; closing it never cancels another run. */
export async function executeStandalone(
  descriptor: NodeDescriptor,
  inputs: Record<string, PortValue>,
  params: Record<string, unknown>,
  onEvent: (event: ProgressMsg) => void,
  signal?: AbortSignal,
): Promise<RunOutcome & { outputs?: Record<string, PortValue>; elapsed?: number }> {
  if (!inTauri) return { status: "error", error: "浏览器预览无法执行节点，请在桌面应用中运行" };
  if (active || foreground) return { status: "error", error: "已有任务正在运行" };
  if (signal?.aborted) return { status: "cancelled" };
  let resolve!: () => void;
  const run: ActiveRun = { standalone: true, job: null, cancelled: false, clear: false, revision: 0,
    finished: new Promise<void>((done) => { resolve = done; }), resolve: () => resolve() };
  active = run;
  pending = false;
  const nodeId = `standalone_${Date.now()}`;
  const state = useRunStore.getState();
  state.setRunning(true); state.setLastError(null);
  const entry = state.startHistory({ scope: "standalone", title: `独立调用：${descriptor.displayName}`, projectName: "模块库", nodeCount: 1, edgeCount: 0, nodes: [] });
  const abort = () => { if (active === run) void cancelActive(false); };
  signal?.addEventListener("abort", abort, { once: true });
  const start = Date.now();
  let result: RunOutcome & { outputs?: Record<string, PortValue>; elapsed?: number } = { status: "success" };
  try {
    const outputs = await api.runNodeStreamed(descriptor.id, nodeId, inputs, params, (event) => {
      if (active !== run) return;
      if (event.kind === "jobStarted") {
        run.job = event.job;
        if (run.cancelled) void api.cancelJob(event.job).catch(() => {});
      }
      if (run.cancelled) return;
      if (event.kind === "nodeFailed" || event.kind === "jobFailed") run.failure = event.error;
      useRunStore.getState().recordProgress(event);
      onEvent(event);
    });
    result = run.cancelled ? { status: "cancelled" } : run.failure ? { status: "error", error: run.failure } : { status: "success", outputs };
  } catch (e) { result = run.cancelled ? { status: "cancelled" } : { status: "error", error: errorText(e) }; }
  finally {
    if (run.cancelled) result = { status: "cancelled" };
    result.elapsed = Date.now() - start;
    signal?.removeEventListener("abort", abort);
    useRunStore.getState().finishHistory(entry, { status: result.status, elapsed: result.elapsed, error: result.error,
      nodes: [{ id: nodeId, descriptorId: descriptor.id, label: descriptor.displayName, status: result.status, error: result.error, outputs: sanitizeOutputs(result.outputs) }] });
    active = null; pending = false;
    useRunStore.getState().setRunning(false);
    useRunStore.getState().setElapsed(result.elapsed);
    run.resolve();
  }
  return result;
}
