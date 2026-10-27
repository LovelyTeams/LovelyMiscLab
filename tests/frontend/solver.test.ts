import { beforeEach, test } from "node:test";
import assert from "node:assert/strict";
import catalog from "../../src/lib/nodeCatalog.generated.json";
import { api } from "@/lib/bindings";
import { useDescriptorStore as D } from "@/store/descriptors";
import { useGraphStore as G } from "@/store/graph";
import { useRunStore as R } from "@/store/run";
import { useAgentStore as A } from "@/store/agent";
import { runAgent } from "@/flow/agentRunner";
import { executeGraph, executeStandalone, stopRun } from "@/flow/runner";
import { prepareSolvePlan, solveCandidates, solveEvidence, solveFingerprint, solvePrompt } from "@/flow/agentSolve";
import { planSolveRound } from "@/flow/agentSolvePlanner";
import { solveChanges, solveRepairChecks } from "@/flow/agentSolveChanges";
import { searchNodes } from "@/lib/catalog";
import { findSolverModules, solverModuleIndex } from "@/flow/agentModuleSearch";

const options = () => ({ task: "解出原始密文", data: "uryyb", files: [], prefix: "flag{", expected: "", maxRounds: 3, maxMinutes: 1 });
const node = (key, descriptorId, params = {}) => ({ key, descriptorId, params, position: { x: 0, y: 0 } });
const edge = (from, output, to, input) => ({ from: { node: from, port: output }, to: { node: to, port: input } });
const plan = (id = "rot13", params = {}) => ({ notes: `尝试 ${id}`, nodes: [node("input", "text_input", { text: "" }), node("try", id, params)], edges: [edge("input", "text", "try", "text")] });
const reviewed = (graph = plan(), solve = options()) => ({ graph, solve, revision: G.getState().runRevision, replace: false });
const gate = () => { let resolve; const promise = new Promise((r) => resolve = r); return { promise, resolve }; };
const success = (graph, emit, text) => {
  const out = {};
  for (const n of graph.nodes) { emit({ kind: "nodeDone", node: n.id }); out[n.id] = { text: { type: "text", value: n.descriptorId === "text_input" ? n.params.text : text } }; }
  return out;
};
beforeEach(() => {
  D.getState().setDescriptors(catalog);
  G.setState({ nodes: [], edges: [], past: [], future: [], selectedId: null, runRevision: 0, suppressHistory: false, historyGroup: null });
  R.setState({ mode: "idle", running: false, history: [], activeHistoryId: null, lastError: null });
  A.setState({ running: false, cancelling: false, cancelled: false, steps: [], notes: "", error: "", pendingPlan: null, solveStatus: "idle", candidates: [], attempts: [], solveProgress: null });
  api.resetRun = async () => {}; api.cancelJob = async () => {};
  api.decideSolverStep = async () => ({ observation: "根据实际输出继续", action: "extend", nextGoal: "查找字符变换", queries: ["caesar", "rot13"], repairs: [] });
});

test("one-click solve shows its first decision and module search before automatically building and executing", async () => {
  const decided = gate(), planning = gate(), generated = gate(); let calls = 0;
  const stages = [];
  const off = A.subscribe((s) => { if (s.solveProgress) stages.push(s.solveProgress.stage); });
  api.decideSolverStep = async () => { decided.resolve(); return { observation: "文本呈现字母替换特征", action: "inspect", nextGoal: "尝试 ROT13 并核验输出", queries: ["rot13"], repairs: [] }; };
  api.generateWorkflow = async () => { planning.resolve(); return generated.promise; };
  api.runGraph = async (graph, emit) => { calls++; return success(graph, emit, "flag{one_click}"); };
  const task = runAgent({ solve: { ...options(), expected: "flag{one_click}" }, replace: false, revision: G.getState().runRevision });
  await decided.promise;
  assert.equal(A.getState().running, true); assert.equal(R.getState().running, true);
  await planning.promise;
  assert.equal(G.getState().nodes.length, 0);
  assert.equal(A.getState().solveProgress.stage, "planning");
  assert.equal(A.getState().solveProgress.planning.decision.nextGoal, "尝试 ROT13 并核验输出");
  assert.ok(A.getState().solveProgress.planning.searches[0].matches.some((m) => m.id === "rot13"));
  assert.equal((await executeGraph()).status, "error");
  generated.resolve(plan()); await task; off();
  assert.equal(calls, 1); assert.equal(G.getState().nodes.length, 2);
  assert.equal(A.getState().solveStatus, "verified"); assert.equal(A.getState().solveProgress.stage, "finished");
  assert.equal(A.getState().solveProgress.plan.nodes.length, 2);
  for (const stage of ["analyzing", "planning", "building", "running", "checking", "finished"]) assert.ok(stages.includes(stage));
  G.getState().undo(); assert.equal(G.getState().nodes.length, 0);
});

test("one-click solve cancels during its first decision and drops late planning events", async () => {
  const pending = gate(), started = gate(), jobs = []; let emit, generated = 0;
  api.decideSolverStep = async (_prompt, cb) => { emit = cb; started.resolve(); return pending.promise; };
  api.generateWorkflow = async () => { generated++; return plan(); };
  api.cancelJob = async (job) => jobs.push(job);
  const task = runAgent({ solve: options(), replace: true, revision: G.getState().runRevision });
  await started.promise; await stopRun(); await task;
  const progress = A.getState().solveProgress;
  emit({ kind: "jobStarted", job: "first-decision-late" }); emit({ kind: "log", message: "迟到的规划状态" });
  pending.resolve({ observation: "late", action: "inspect", nextGoal: "late", queries: ["rot13"], repairs: [] });
  await Promise.resolve();
  assert.equal(A.getState().solveStatus, "cancelled"); assert.equal(R.getState().running, false);
  assert.equal(generated, 0); assert.equal(G.getState().nodes.length, 0);
  assert.deepEqual(jobs, ["first-decision-late"]); assert.deepEqual(A.getState().solveProgress, progress);
});

for (const action of ["cancel", "edit"]) test(`first-plan ${action} preserves the existing canvas and rejects the late graph`, async () => {
  const id = G.getState().addNode(D.getState().byId.text_input, { x: 0, y: 0 });
  G.getState().setParam(id, "text", "existing");
  const pending = gate(), started = gate(); let executions = 0;
  api.generateWorkflow = async () => { started.resolve(); return pending.promise; };
  api.runGraph = async () => { executions++; return {}; };
  const task = runAgent({ solve: options(), replace: true, revision: G.getState().runRevision });
  await started.promise;
  if (action === "cancel") await stopRun();
  else G.getState().setParam(id, "text", "user edit");
  pending.resolve(plan()); await task;
  assert.equal(executions, 0); assert.equal(G.getState().nodes.length, 1); assert.equal(G.getState().nodes[0].id, id);
  assert.equal(G.getState().nodes[0].data.params.text, action === "edit" ? "user edit" : "existing");
  assert.equal(A.getState().solveStatus, "cancelled");
});

test("one-click requests freeze scope, reject duplicate starts and validate before contacting the model", async () => {
  const request = { solve: options(), replace: false, revision: G.getState().runRevision };
  assert.equal(A.getState().launchSolve(request), true); assert.equal(A.getState().launchSolve(request), false);
  request.solve.task = "mutated"; request.solve.files.push("D:/not-approved.dat");
  assert.equal(A.getState().pendingPlan.solve.task, "解出原始密文"); assert.deepEqual(A.getState().pendingPlan.solve.files, []);
  A.getState().requestCancel(); assert.equal(A.getState().pendingPlan, null);
  let calls = 0; api.decideSolverStep = async () => { calls++; throw Error("must not be called"); };
  await runAgent({ solve: { ...options(), maxRounds: 0 }, replace: false, revision: G.getState().runRevision });
  assert.equal(calls, 0); assert.equal(A.getState().solveStatus, "unresolved"); assert.equal(G.getState().nodes.length, 0);
});

test("solver uses real intermediate evidence to change parameters; candidates are separate from solved", async () => {
  let calls = 0, prompt = "";
  api.runGraph = async (graph, emit) => success(graph, emit, ++calls === 1 ? "intermediate-rot13-no-flag" : "flag{真实_42}");
  api.generateWorkflow = async (text) => { prompt = text; assert.equal(R.getState().running, true); return plan("caesar", { amount: 7 }); };
  await runAgent(reviewed());
  assert.equal(calls, 2); assert.match(prompt, /intermediate-rot13-no-flag/);
  assert.equal(A.getState().solveStatus, "candidate"); assert.equal(A.getState().candidates[0].verified, false);
  assert.equal(A.getState().attempts.length, 2); assert.equal(G.getState().nodes.length, 2);
  assert.equal(G.getState().nodes.find((n) => n.data.descriptorId === "caesar").data.params.amount, 7); assert.equal(R.getState().running, false);
  G.getState().undo(); assert.equal(G.getState().nodes.length, 2);
  G.getState().undo(); assert.equal(G.getState().nodes.length, 0);
});

test("all successful nodes without a target remain unresolved; renamed duplicate plans do not loop", async () => {
  let runs = 0;
  api.runGraph = async (graph, emit) => { runs++; return success(graph, emit, "hello"); };
  api.generateWorkflow = async () => {
    const p = plan(); p.nodes[1].key = "renamed"; p.edges[0].to.node = "renamed"; p.nodes.reverse(); return p;
  };
  await runAgent(reviewed());
  assert.equal(runs, 1); assert.equal(A.getState().solveStatus, "unresolved"); assert.match(A.getState().notes, /重复/);
  assert.equal(A.getState().candidates.length, 0);
});

test("planning gaps hold the foreground lock; cancellation drops late plans and preserves unrelated results", async () => {
  const old = G.getState().addNode(D.getState().byId.text_input, { x: -500, y: 0 });
  G.getState().updateRuntime(old, { status: "done", outputs: { text: { type: "text", value: "keep" } } });
  const pending = gate(), planning = gate(), jobs = []; let emit;
  api.runGraph = async (graph, cb) => success(graph, cb, "no target");
  api.generateWorkflow = async (_prompt, cb) => { emit = cb; planning.resolve(); return pending.promise; };
  api.cancelJob = async (id) => jobs.push(id);
  const task = runAgent(reviewed()); await planning.promise;
  assert.equal((await executeGraph()).status, "error");
  assert.equal((await executeStandalone(D.getState().byId.text_input, {}, {}, () => {})).status, "error");
  await stopRun(); emit({ kind: "jobStarted", job: "late-solver-plan" });
  pending.resolve(plan("caesar", { amount: 1 })); await task;
  assert.equal(G.getState().nodes.length, 3); assert.equal(A.getState().solveStatus, "cancelled");
  assert.deepEqual(jobs, ["late-solver-plan"]);
  assert.equal(G.getState().nodes.find((n) => n.id === old).data.outputs.text.value, "keep");
  assert.equal(R.getState().running, false);
});

test("canvas changes while replanning cancel the round and cannot apply the late graph", async () => {
  const pending = gate(), planning = gate();
  api.runGraph = async (graph, emit) => success(graph, emit, "no target");
  api.generateWorkflow = async () => { planning.resolve(); return pending.promise; };
  const task = runAgent(reviewed()); await planning.promise;
  G.getState().setParam(G.getState().nodes[0].id, "text", "user edit");
  pending.resolve(plan("caesar", { amount: 1 })); await task;
  assert.equal(G.getState().nodes.length, 2); assert.equal(A.getState().solveStatus, "cancelled"); assert.match(A.getState().notes, /画布已修改/);
});

test("node errors and partial outputs feed the next round, within the approved limit", async () => {
  let runs = 0;
  api.runGraph = async (graph, emit) => {
    runs++; emit({ kind: "nodeFailed", node: graph.nodes[1].id, error: "invalid padding observed" });
    return { [graph.nodes[0].id]: { text: { type: "text", value: "input" } } };
  };
  api.generateWorkflow = async (prompt) => { assert.match(prompt, /invalid padding observed/); return plan("caesar", { amount: 1 }); };
  await runAgent(reviewed(plan(), { ...options(), maxRounds: 2 }));
  assert.equal(runs, 2); assert.equal(A.getState().solveStatus, "unresolved"); assert.match(A.getState().notes, /2 轮/);
  assert.ok(A.getState().attempts.every((a) => a.error));
});

test("file, archive, image and audio plans use only selected input paths and concrete byte ports", () => {
  for (const id of ["detect_file_type", "zsteg_scan", "wav_lsb", "audio_info", "image_info", "archive_extract"]) {
    const opts = { ...options(), data: "", files: ["D:/challenge/file.bin"] };
    const p = { notes: "文件", nodes: [node("in", "file_import", { path: opts.files[0] }), node("work", id)],
      edges: [edge("in", "bytes", "work", id === "archive_extract" ? "archive" : "data")] };
    assert.doesNotThrow(() => prepareSolvePlan(p, D.getState().byId, opts));
    p.nodes[0].params.path = "D:/private.txt";
    assert.throws(() => prepareSolvePlan(p, D.getState().byId, opts), /题目文件/);
    if (id !== "detect_file_type") {
      p.nodes[0].params.path = opts.files[0]; p.edges[0].from.port = "path";
      assert.throws(() => prepareSolvePlan(p, D.getState().byId, opts), /字节/);
    }
  }
  const p = plan(); p.nodes.push(node("net", "http_request"));
  assert.throws(() => prepareSolvePlan(p, D.getState().byId, options()), /自主解题/);
});

test("candidate extraction preserves evidence, rejects parameter-injected flags and does not expose held-out answer", () => {
  const p = prepareSolvePlan(plan("caesar", { amount: 2 }), D.getState().byId, options());
  const answer = "flag{真实答案}";
  const results = { try: { status: "done", outputs: { bytes: { type: "bytes", value: [...new TextEncoder().encode(answer)] } } } };
  const opts = { ...options(), expected: answer };
  assert.equal(solveCandidates(p, results, opts, 2)[0].verified, true);
  assert.deepEqual(solveCandidates(p, results, opts, 2)[0].steps, ["text_input", "caesar"]);
  assert.ok(!solvePrompt(opts).includes(answer));
  p.nodes[1].params.fake = answer;
  assert.deepEqual(solveCandidates(p, results, opts, 2), []);
  assert.ok(JSON.stringify(solveEvidence(p, { try: { status: "done", outputs: { big: { type: "bytes", value: new Array(100000).fill(65) } } } })).length < 6000);
});

test("equivalent Windows input spelling is restored to the approved path without widening file scope", () => {
  const approved = "D:\\challenge\\complex\\specimen.dat";
  const opts = { ...options(), data: "", files: [approved] };
  const p = { notes: "文件", nodes: [node("in", "file_import", { path: "d:/challenge/complex/specimen.dat" }), node("work", "archive_extract")],
    edges: [edge("in", "bytes", "work", "archive")] };
  assert.equal(prepareSolvePlan(p, D.getState().byId, opts).nodes[0].params.path, approved);
  p.nodes[0].params.path = "";
  assert.equal(prepareSolvePlan(p, D.getState().byId, opts).nodes[0].params.path, approved);
  assert.throws(() => prepareSolvePlan(p, D.getState().byId, { ...opts, files: [approved, "D:/another.dat"] }), /只能读取本次选择/);
  for (const path of ["D:/challenge/complex/packet.json", "D:/challenge/complex/../complex/specimen.dat", "D:/challenge/Complex/specimen.dat", "D:challenge/complex/specimen.dat", "/challenge/complex/specimen.dat"] ) {
    p.nodes[0].params.path = path;
    assert.throws(() => prepareSolvePlan(p, D.getState().byId, opts), /只能读取本次选择/);
  }
});

test("solver validates changed review parameters again and exact user answers yield verified", async () => {
  let calls = 0;
  api.runGraph = async (graph, emit) => { calls++; return success(graph, emit, "flag{test}"); };
  await runAgent(reviewed(plan("caesar", { amount: 9999 })));
  assert.equal(calls, 0); assert.equal(G.getState().nodes.length, 0); assert.equal(A.getState().solveStatus, "unresolved");
  await runAgent(reviewed(plan(), { ...options(), expected: "flag{test}" }));
  assert.equal(A.getState().solveStatus, "verified"); assert.equal(calls, 1);
  const p = prepareSolvePlan(plan(), D.getState().byId, options());
  assert.equal(solveFingerprint(p), solveFingerprint({ ...p, nodes: [...p.nodes].reverse() }));
});

test("a wrong candidate feeds back a failed local check without leaking the held-out answer", async () => {
  const opts = { ...options(), expected: "flag{held_out_solution}" }; let calls = 0;
  api.runGraph = async (graph, emit) => success(graph, emit, ++calls === 1 ? "flag{wrong_candidate}" : opts.expected);
  api.generateWorkflow = async (prompt) => {
    assert.match(prompt, /未通过本地标准答案校验/);
    assert.ok(!prompt.includes(opts.expected));
    return plan("caesar", { amount: 1 });
  };
  await runAgent(reviewed(plan(), opts));
  assert.equal(calls, 2); assert.equal(A.getState().solveStatus, "verified");
  assert.ok(A.getState().attempts[0].error.includes("未通过本地标准答案"));
  assert.equal(A.getState().candidates.filter((c) => c.verified).length, 1);
});

test("solver diagnoses, searches real modules, edits the same nodes, retests and supports undo", async () => {
  let runs = 0, originalIds;
  const p = plan("caesar", { amount: 11 }); p.checks = [{ node: "try", port: "text", kind: "contains", expected: "flag{" }];
  const old = G.getState().addNode(D.getState().byId.text_input, { x: -700, y: 0 });
  G.getState().setParam(old, "text", "unrelated work");
  api.runGraph = async (graph, emit) => {
    if (!runs++) { originalIds = graph.nodes.map((n) => n.id); return success(graph, emit, "wrong shift"); }
    assert.deepEqual(graph.nodes.map((n) => n.id), originalIds);
    assert.equal(graph.nodes[1].params.amount, 15);
    return success(graph, emit, "flag{repaired}");
  };
  api.decideSolverStep = async (prompt) => {
    assert.match(prompt, /wrong shift/);
    return { observation: "输出没有命中目标", action: "repair", nextGoal: "调整位移后复测", queries: ["caesar"],
      repairs: [{ node: "try", problem: "位移错误", change: "将 amount 调整为 15", verify: "结果包含目标前缀" }] };
  };
  api.generateWorkflow = async (prompt, _emit, ids) => {
    assert.match(prompt, /凯撒/); assert.ok(ids.includes("caesar")); assert.ok(!ids.includes("http_request"));
    const next = structuredClone(p); next.nodes[1].params.amount = 15; return next;
  };
  await runAgent(reviewed(p));
  assert.equal(A.getState().solveStatus, "candidate");
  assert.equal(G.getState().nodes.length, 3);
  assert.ok(A.getState().steps.some((s) => s.kind === "search" && s.text.includes("caesar")));
  assert.ok(A.getState().attempts[1].changes.some((s) => s.includes("11 → 15")));
  assert.equal(A.getState().attempts[1].repairChecks[0].status, "passed");
  assert.equal(G.getState().nodes.find((n) => n.id === old).data.params.text, "unrelated work");
  G.getState().undo(); assert.equal(G.getState().nodes.find((n) => n.id === originalIds[1]).data.params.amount, 11);
});

test("unavailable modules stop before graph generation; decision-stage cancellation discards late work", async () => {
  let generated = 0;
  api.decideSolverStep = async () => ({ observation: "需要未知处理", action: "extend", nextGoal: "查询不存在能力", queries: ["__missing_capability__"], repairs: [] });
  api.generateWorkflow = async () => { generated++; return plan(); };
  await assert.rejects(planSolveRound(options(), [], undefined, D.getState().byId, new AbortController().signal, () => {}), /未找到可用模块/);
  assert.equal(generated, 0);
  const pending = gate(), started = gate(), jobs = []; let emit;
  api.runGraph = async (graph, cb) => success(graph, cb, "no target");
  api.decideSolverStep = async (_prompt, cb) => { emit = cb; started.resolve(); return pending.promise; };
  api.cancelJob = async (job) => jobs.push(job);
  const work = runAgent(reviewed()); await started.promise; await stopRun();
  emit({ kind: "jobStarted", job: "late-decision" });
  pending.resolve({ observation: "late", action: "extend", nextGoal: "late", queries: ["caesar"], repairs: [] });
  await work;
  assert.equal(A.getState().solveStatus, "cancelled"); assert.equal(generated, 0); assert.deepEqual(jobs, ["late-decision"]);
});

test("repair verification does not equate deleted nodes or successful execution with a verified fix", () => {
  const p = plan(), planning = { decision: { repairs: [{ node: "try" }] } };
  const results = { try: { status: "done", outputs: { text: { type: "text", value: "not target" } } } };
  assert.equal(solveRepairChecks(planning, p, results)[0].status, "executed");
  p.checks = [{ node: "try", port: "text", kind: "contains", expected: "flag{" }];
  assert.equal(solveRepairChecks(planning, p, results)[0].status, "failed");
  assert.equal(solveRepairChecks(planning, { ...p, nodes: p.nodes.slice(0, 1) }, results)[0].status, "removed");
  const modified = structuredClone(p); modified.nodes[1].params.fake = "value";
  assert.deepEqual(solveChanges(p, p, D.getState().byId), []);
});

test("large early outputs cannot hide later evidence from the next decision", () => {
  const evidence = Array.from({ length: 20 }, (_, i) => ({ node: `n${i}`, status: "done", outputs: i === 19 ? "KEY_HEX=actual_late_evidence" : "x".repeat(5500) }));
  const prompt = solvePrompt(options(), [{ round: 1, notes: "", graph: {}, evidence }]);
  assert.ok(prompt.includes("KEY_HEX=actual_late_evidence"));
  assert.ok(prompt.length < 50000);
});

test("shared module search handles bilingual modes, separators, aliases and full-width query text", () => {
  for (const [query, id] of [["Base64 decode", "base64_decode"], ["Ｂａｓｅ６４　解码模块", "base64_decode"],
    ["AES CBC 解密", "aes"], ["aes_decrypt", "aes"], ["正则表达式提取", "regex_extract"],
    ["archive extraction", "archive_extract"], ["LSB extraction", "lsb_extract"]])
    assert.ok(searchNodes(catalog, query).slice(0, 8).some((d) => d.id === id), `${query} -> ${id}`);
  assert.equal(searchNodes(catalog, "base64_decode")[0].id, "base64_decode");
  assert.ok(!searchNodes(catalog, "base64_decode").some((d) => d.id === "base64_encode"));
  assert.ok(searchNodes(catalog, "AES").some((d) => d.id === "cryptojs_aes"));
  for (const query of ["AES", "ＡＥＳ", "AES decrypt"])
    assert.ok(!searchNodes(catalog, query).some((d) => d.id.startsWith("caesar")), query);
  assert.ok(searchNodes(catalog, "caes").some((d) => d.id === "caesar"));
  assert.ok(searchNodes(catalog, "SHA").some((d) => d.id === "hash"));
  assert.equal(searchNodes(catalog, "__missing_capability__").length, 0);
  const partial = findSolverModules(catalog, "base64 unknownvariant");
  assert.equal(partial.matches.length, 0); assert.ok(partial.suggestions.some((d) => d.id === "base64_decode"));
  const textSuggestions = findSolverModules(catalog, "text output missingalias").suggestions;
  assert.equal(textSuggestions[0].id, "text_output"); assert.ok(!textSuggestions.some((d) => d.id === "aes"));
  const index = solverModuleIndex(catalog); assert.ok(index.includes("aes=AES")); assert.ok(index.length < 30000);
});

test("partial module misses are retried against real catalog IDs instead of silently proceeding", async () => {
  let decisions = 0, generated = 0; const observed = [];
  api.decideSolverStep = async (prompt) => {
    assert.match(prompt, /base64_decode=Base64 解码/); assert.ok(!prompt.includes("http_request="));
    if (++decisions === 1) return { observation: "需要旋转和编码处理", action: "inspect", nextGoal: "查找两项能力", queries: ["rot13", "base64 unknownvariant"], repairs: [] };
    assert.match(prompt, /未命中词：base64 unknownvariant/);
    return { observation: "根据目录改用实际模块", action: "inspect", nextGoal: "先旋转文本", queries: ["rot13"], repairs: [] };
  };
  api.generateWorkflow = async (_prompt, _emit, ids) => { generated++; assert.ok(ids.includes("rot13")); assert.ok(ids.length <= 100); return plan(); };
  const result = await planSolveRound(options(), [], undefined, D.getState().byId, new AbortController().signal, () => {}, (p) => observed.push(p));
  assert.equal(decisions, 2); assert.equal(generated, 1);
  assert.equal(result.graph.nodes.length, 2); assert.equal(observed[0].searches[1].matches.length, 0);
  assert.ok(result.planning.searchHistory.some((s) => s.query === "base64 unknownvariant" && s.suggestions.length));
  assert.deepEqual(result.planning.searches.map((s) => s.query), ["rot13"]);
});

test("manual network retry resumes second-round planning without duplicating completed rounds or nodes", { timeout: 10000 }, async () => {
  let runs = 0, decisions = 0, generations = 0; const waiting = gate();
  const off = A.subscribe((s) => { if (s.retryPrompt) waiting.resolve(); });
  api.runGraph = async (graph, cb) => success(graph, cb, ++runs === 1 ? "intermediate" : "flag{resumed}");
  api.decideSolverStep = async () => { decisions++; return { observation: "继续处理", action: "extend", nextGoal: "调整位移", queries: ["caesar"], repairs: [] }; };
  api.generateWorkflow = async () => {
    if (++generations <= 3) throw { code: "ai_response", message: "模型服务 JSON 被截断", retryable: true };
    return plan("caesar", { amount: 1 });
  };
  const task = runAgent(reviewed()); await waiting.promise;
  assert.equal(runs, 1); assert.equal(A.getState().attempts.length, 1); assert.equal(R.getState().running, true);
  assert.equal(G.getState().nodes.length, 2); const source = G.getState().nodes[0].id;
  A.getState().retryAI(); await task; off();
  assert.equal(generations, 4); assert.equal(decisions, 1); assert.equal(runs, 2);
  assert.equal(G.getState().nodes.length, 2); assert.equal(G.getState().nodes[0].id, source);
  assert.equal(A.getState().solveStatus, "candidate"); assert.equal(A.getState().retryPrompt, null);
});
