import { beforeEach, test } from "node:test";
import assert from "node:assert/strict";
import catalog from "../../src/lib/nodeCatalog.generated.json";
import { api } from "@/lib/bindings";
import { useDescriptorStore as D } from "@/store/descriptors";
import { useGraphStore as G } from "@/store/graph";
import { useRunStore as R } from "@/store/run";
import { useAgentStore as A } from "@/store/agent";
import { agentRequest } from "@/flow/agentRequest";
import { preparePlan, withExpectedText } from "@/flow/agentPlan";
import { checkAgentResults } from "@/flow/agentChecks";
import { runAgent } from "@/flow/agentRunner";
import { stopRun } from "@/flow/runner";

const gate = () => { let resolve; const promise = new Promise((r) => resolve = r); return { promise, resolve }; };
const plan = () => preparePlan({ notes: "Base64 解码", nodes: [
  { key: "source", descriptorId: "text_input", params: { text: "SGVsbG8=" }, position: { x: 40, y: 60 } },
  { key: "decode", descriptorId: "base64_decode", params: {}, position: { x: 300, y: 60 } },
  { key: "out", descriptorId: "text_output", params: {}, position: { x: 560, y: 60 } },
], edges: [{ from: { node: "source", port: "text" }, to: { node: "decode", port: "text" } },
  { from: { node: "decode", port: "text" }, to: { node: "out", port: "text" } }],
  checks: [{ node: "out", port: "value", kind: "equals", expected: "Hello" }],
}, D.getState().byId);
beforeEach(() => {
  D.getState().setDescriptors(catalog);
  G.setState({ nodes: [], edges: [], past: [], future: [], selectedId: null, runRevision: 0, suppressHistory: false, historyGroup: null });
  R.setState({ mode: "idle", running: false, history: [], activeHistoryId: null, lastError: null });
  A.setState({ running: false, cancelling: false, cancelled: false, steps: [], notes: "", error: "", pendingPlan: null });
  api.resetRun = async () => {};
  api.cancelJob = async () => {};
});
test("planning cancellation before job ID cancels its own backend and suppresses late progress/result", async () => {
  const controller = new AbortController(), pending = gate(), jobs = [], progress = []; let emit;
  api.cancelJob = async (id) => jobs.push(id);
  const task = agentRequest((callback) => { emit = callback; return pending.promise; }, controller.signal, (msg) => progress.push(msg));
  controller.abort(); emit({ kind: "jobStarted", job: "old-plan" }); emit({ kind: "log", message: "late" });
  pending.resolve(plan()); await assert.rejects(task, /取消/);
  assert.deepEqual(jobs, ["old-plan"]); assert.deepEqual(progress, []); assert.equal(G.getState().nodes.length, 0);
});
test("cancelled old planning request cannot cancel a subsequent plan", async () => {
  const old = new AbortController(), newer = new AbortController(), pending = gate(), jobs = [];
  api.cancelJob = async (id) => jobs.push(id);
  const task = agentRequest((emit) => { emit({ kind: "jobStarted", job: "old" }); return pending.promise; }, old.signal, () => {});
  old.abort();
  const result = await agentRequest(async (emit) => { emit({ kind: "jobStarted", job: "new" }); return plan(); }, newer.signal, () => {});
  pending.resolve(plan()); await assert.rejects(task);
  assert.equal(result.nodes.length, 3); assert.deepEqual(jobs, ["old"]);
});
test("review includes defaults and exact test expectation without modifying canvas or input whitespace", () => {
  const reviewed = withExpectedText(preparePlan(plan(), D.getState().byId, " SGVsbG8=\n"), "Hello\n", D.getState().byId);
  assert.equal(reviewed.nodes[0].params.text, " SGVsbG8=\n");
  for (const spec of D.getState().byId.base64_decode.params) assert.deepEqual(reviewed.nodes[1].params[spec.name], spec.default);
  assert.equal(reviewed.checks[0].expected, "Hello\n"); assert.equal(G.getState().nodes.length, 0);
  for (const check of [{ node: "missing", port: "text", kind: "equals", expected: "x" },
    { node: "out", port: "value", kind: "unknown" }, { node: "out", port: "value", kind: "equals", expected: 42 }]) {
    assert.throws(() => preparePlan({ ...plan(), checks: [check] }, D.getState().byId));
  }
});
function successTransport(text = "Hello") {
  api.runGraph = async (graph, emit) => {
    assert.equal(graph.nodes[0].params.text, "SGVsbG8=");
    const results = {};
    for (const n of graph.nodes) { emit({ kind: "nodeEntered", node: n.id }); emit({ kind: "nodeDone", node: n.id });
      results[n.id] = { [n.descriptorId === "text_output" ? "value" : "text"]: { type: "text", value: text } }; }
    return results;
  };
}
test("confirmed plan configures nodes, wires graph, runs tests and records report as one undo", async () => {
  successTransport();
  await runAgent({ graph: plan(), replace: false, revision: 0 });
  assert.equal(A.getState().error, ""); assert.equal(G.getState().nodes.length, 3); assert.equal(G.getState().edges.length, 2);
  assert.equal(A.getState().steps.filter((s) => s.kind === "result" && s.ok).length, 4);
  assert.ok(R.getState().history[0].events.some((e) => e.message.includes("精确匹配：通过")));
  G.getState().undo(); assert.equal(G.getState().nodes.length, 0);
});
test("a result mismatch fails the agent report and run history without changing approved parameters", async () => {
  successTransport("Wrong");
  await runAgent({ graph: plan(), replace: false, revision: 0 });
  assert.match(A.getState().error, /自动测试 1 项未通过/); assert.equal(R.getState().history[0].status, "error");
  assert.equal(G.getState().nodes[0].data.params.text, "SGVsbG8=");
});
test("missing node results and non-text assertions cannot pass", () => {
  assert.equal(checkAgentResults(plan(), {}).filter((s) => !s.ok).length, 4);
  assert.equal(checkAgentResults(plan(), { out: { status: "done", outputs: { value: { type: "bytes", value: [72] } } } }).at(-1).ok, false);
});
test("agent cancellation retains its graph, discards late output and skips assertions", async () => {
  const pending = gate(), cancelled = []; let emit;
  api.cancelJob = async (id) => cancelled.push(id);
  api.runGraph = async (_graph, callback) => { emit = callback; return pending.promise; };
  const task = runAgent({ graph: plan(), replace: false, revision: 0 });
  const stopped = stopRun(); emit({ kind: "jobStarted", job: "late-run" });
  pending.resolve({}); await task; await stopped;
  assert.equal(A.getState().cancelled, true); assert.equal(A.getState().running, false);
  assert.equal(A.getState().steps.some((s) => s.kind === "result"), false);
  assert.equal(G.getState().nodes.length, 3); assert.deepEqual(cancelled, ["late-run"]);
  assert.equal(R.getState().history[0].status, "cancelled");
});

test("cancelling an appended AI plan preserves results outside that plan", async () => {
  const existing = G.getState().addNode(D.getState().byId.text_input, { x: 0, y: 0 });
  G.getState().updateRuntime(existing, { status: "done", outputs: { text: { type: "text", value: "keep this result" } }, progress: 1 });
  const pending = gate(); let sent, emit;
  api.runGraph = async (graph, callback) => { sent = graph; emit = callback; callback({ kind: "jobStarted", job: "appended-plan" }); return pending.promise; };
  const task = runAgent({ graph: plan(), replace: false, revision: G.getState().runRevision });
  const stopped = stopRun();
  emit({ kind: "nodeDone", node: sent.nodes[0].id });
  pending.resolve({ [sent.nodes[0].id]: { text: { type: "text", value: "late result" } } });
  await task; await stopped;
  const oldNode = G.getState().nodes.find((n) => n.id === existing);
  assert.equal(oldNode.data.status, "done");
  assert.equal(oldNode.data.outputs.text.value, "keep this result");
  assert.ok(G.getState().nodes.filter((n) => n.id !== existing).every((n) => n.data.status === "idle" && n.data.outputs === undefined));
  assert.equal(R.getState().history[0].nodes.some((n) => n.id === existing), false);
});

test("AI branch layout accommodates growing previews and preserves unrelated nodes, results and one-step undo", async () => {
  const existing = G.getState().addNode(D.getState().byId.text_input, { x: -400, y: 60 });
  G.getState().updateRuntime(existing, { status: "done", outputs: { text: { type: "text", value: "existing" } } });
  const branched = plan();
  branched.nodes.push({ ...structuredClone(branched.nodes[1]), key: "parallel", position: { x: 300, y: 210 } });
  branched.edges.push({ from: { node: "source", port: "text" }, to: { node: "parallel", port: "text" } });
  successTransport();
  await runAgent({ graph: branched, replace: false, revision: G.getState().runRevision });
  const ids = new Set(G.getState().nodes.filter((n) => n.id !== existing).map((n) => n.id));
  const before = G.getState(), oldNode = before.nodes.find((n) => n.id === existing);
  const branches = before.nodes.filter((n) => n.data.descriptorId === "base64_decode");
  G.getState().onNodesChange([...ids].map((id, index) => ({ type: "dimensions", id, dimensions: { width: index === 0 ? 400 : 200, height: 450 } })));
  const assertNoOverlap = () => {
    const current = G.getState().nodes.filter((n) => ids.has(n.id));
    for (const [i, a] of current.entries()) for (const b of current.slice(i + 1)) {
      assert.ok(a.position.x + a.measured.width <= b.position.x || b.position.x + b.measured.width <= a.position.x ||
        a.position.y + a.measured.height <= b.position.y || b.position.y + b.measured.height <= a.position.y,
        `${a.id} overlaps ${b.id}`);
    }
  };
  G.getState().layoutAgentNodes(ids);
  assertNoOverlap();
  G.getState().onNodesChange([{ type: "dimensions", id: branches[0].id, dimensions: { width: 200, height: 920 } }]);
  G.getState().layoutAgentNodes(ids);
  assertNoOverlap();
  assert.equal(G.getState().layoutAgentNodes(ids), false);
  assert.equal(G.getState().nodes.find((n) => n.id === existing), oldNode);
  assert.equal(G.getState().runRevision, before.runRevision);
  assert.equal(G.getState().past.length, before.past.length);
  assert.ok(G.getState().nodes.every((n) => n.data.status === "done" && n.data.outputs));
  G.getState().undo();
  assert.deepEqual(G.getState().nodes.map((n) => n.id), [existing]);
  G.getState().redo();
  assert.equal(G.getState().nodes.length, 5);
  assertNoOverlap();
});
