import { beforeEach, test } from "node:test";
import assert from "node:assert/strict";
import catalog from "../../src/lib/nodeCatalog.generated.json";
import { api } from "@/lib/bindings";
import { useDescriptorStore as D } from "@/store/descriptors";
import { useGraphStore as G } from "@/store/graph";
import { useRunStore as R } from "@/store/run";
import { useAgentStore as A } from "@/store/agent";
import { preparePlan } from "@/flow/agentPlan";
import { runAgent } from "@/flow/agentRunner";
import { executeGraph, executeToNode, executeStandalone, sanitizeValue, runSingleNode, pauseRun, stopRun } from "@/flow/runner";
import { draftFromValue, parseInput, parseInputs } from "@/lib/typedInputs";
import { searchNodes, inlineParams, matchesCategory } from "@/lib/catalog";

const g = () => G.getState();
const node = (id) => g().nodes.find((n) => n.id === id);
const add = (id = "text_input") => g().addNode(D.getState().byId[id], { x: 0, y: 0 });
const connect = (source, target, sourceHandle = "text", targetHandle = "text") =>
  g().onConnect({ source, target, sourceHandle, targetHandle });
const done = (id) => g().updateRuntime(id, { status: "done", outputs: { text: { type: "text", value: "old" } } });
const deferred = () => { let resolve; const promise = new Promise((r) => { resolve = r; }); return { promise, resolve }; };
const makePlan = () => ({ notes: "输入文本并显示", nodes: [
  { key: "input", descriptorId: "text_input", params: { text: "hello" }, position: { x: 0, y: 0 } },
  { key: "out", descriptorId: "text_output", params: {}, position: { x: 250, y: 0 } },
], edges: [{ from: { node: "input", port: "text" }, to: { node: "out", port: "text" } }] });

test("typed standalone inputs preserve binary bytes and omit unset optional ports", () => {
  const port = (type, required = true) => ({ name: "input", label: "输入", type, required });
  assert.deepEqual(parseInput(port("bytes"), { format: "hex", text: "00 ff 41" }), { type: "bytes", value: [0, 255, 65] });
  assert.deepEqual(parseInput(port("bytes"), { format: "base64", text: "AP9B" }), { type: "bytes", value: [0, 255, 65] });
  assert.deepEqual(parseInput(port("any"), { format: "utf8", text: "中" }), { type: "bytes", value: [228, 184, 173] });
  assert.deepEqual(parseInput(port("number"), { text: "12.5" }), { type: "number", value: 12.5 });
  assert.deepEqual(parseInput(port("bool"), { text: "false" }), { type: "bool", value: false });
  assert.deepEqual(parseInput(port("json"), { text: '{"a":[1]}' }), { type: "json", value: { a: [1] } });
  assert.deepEqual(parseInput(port("stringList"), { text: '["a", "b"]' }), { type: "stringList", value: ["a", "b"] });
  assert.deepEqual(parseInputs([port("text", false)], {}), {});
  assert.deepEqual(parseInput(port("text", false), { enabled: true, text: "" }), { type: "text", value: "" });
  for (const [type, draft] of [["bytes", { format: "hex", text: "abc" }], ["bytes", { format: "base64", text: "%%%=" }], ["number", { text: "Infinity" }], ["number", { text: "" }], ["json", { text: "{" }], ["candidates", { text: '[{"text":"a"}]' }], ["artifact", { text: "../../file" }]]) assert.throws(() => parseInput(port(type), draft));
});

test("shared search ranks aliases, IDs and old categories and limits inline fields", () => {
  const list = D.getState().list;
  const exact = { ...list[0], id: "target", aliases: ["B64"], displayName: "匹配" };
  const prefix = { ...exact, id: "prefix", aliases: ["b64-extra"] };
  const substring = { ...exact, id: "contains", aliases: ["x-b64-y"] };
  assert.deepEqual(searchNodes([substring, prefix, exact], " b64 ").map((d) => d.id), ["target", "prefix", "contains"]);
  const base = D.getState().byId.base64_encode;
  assert.ok(matchesCategory(base, "编码/加密"));
  assert.ok(matchesCategory(base, "encoding.base"));
  assert.ok(matchesCategory(base, "Base 编码"));
  assert.ok(searchNodes(list, "传输编码").some((d) => d.id === "http_basic_auth_encode"));
  for (const d of list) if (d.inputs.length) assert.ok(inlineParams(d).length <= 3);
});

test("foundation migration nodes expose searchable categories and typed previews", () => {
  const expected = [
    ["dxBase64", "dx_base64_encode", "encoding.base"],
    ["rison", "rison_decode", "data.serialization"],
    ["unixPerms", "unix_permissions", "numeric.bits"],
    ["uuidParse", "uuid_parse", "numeric.time"],
    ["objectIdTime", "object_id_time", "numeric.time"],
    ["hexRange", "hex_range", "numeric.bits"],
    ["hexStats", "hex_stats", "analysis.statistics"],
    ["bitPlaneExtract", "bit_plane_extract", "numeric.bits"],
    ["geoDms", "geo_dms_encode", "numeric.address"],
    ["geoHash", "geo_hash_decode", "numeric.address"],
    ["geoPlusCode", "geo_plus_code_encode", "numeric.address"],
    ["geoMaidenhead", "geo_maidenhead_decode", "numeric.address"],
    ["geoUtm", "geo_utm_encode", "numeric.address"],
    ["hammingCode", "hamming_code_decode", "numeric.bits"],
    ["huffmanCodec", "huffman_encode", "archive.compression"],
    ["color", "color_convert_encode", "numeric.address"],
    ["colorInfo", "color_info", "numeric.address"],
  ];
  for (const [upstream, id, category] of expected) {
    const d = D.getState().byId[id];
    assert.ok(searchNodes(D.getState().list, upstream).some((node) => node.id === id));
    assert.ok(searchNodes(D.getState().list, d.displayName).some((node) => node.id === id));
    for (const alias of d.aliases) assert.ok(searchNodes(D.getState().list, alias).some((node) => node.id === id));
    assert.equal(d.subcategoryId, category);
    assert.ok(matchesCategory(d, category));
    assert.ok(inlineParams(d).length <= 3);
    for (const p of d.params) assert.ok(p.description.length);
    for (const port of d.outputs) {
      if (port.type === "json") assert.equal(d.presentation.previews[port.name], "json");
      if (port.type === "bytes") assert.equal(d.presentation.previews[port.name], "hex");
    }
  }
});

test("color conversion direction and RGB outputs remain usable in canvas and standalone", async () => {
  const convert = add("color_convert_encode"), info = add("color_info"), stats = add("hex_stats");
  connect(convert, info); connect(info, stats, "bytes", "data"); assert.equal(g().edges.length, 2);
  g().setParam(convert, "from", "hsl"); g().setParam(convert, "to", "hex");
  g().setParam(convert, "upper", false); g().undo(); assert.equal(node(convert).data.params.upper, true);
  const d = D.getState().byId.color_convert_decode;
  assert.deepEqual(inlineParams(d).map((p) => p.name), ["from", "to", "upper"]);
  const example = d.examples[0]; const before = JSON.stringify(g());
  api.runNodeStreamed = async (id, _nodeId, input, _params, emit) => {
    assert.equal(id, "color_convert_decode"); assert.deepEqual(input, example.inputs);
    emit({ kind: "jobStarted", job: "color-standalone" });
    return { bytes: { type: "bytes", value: [255, 0, 0] }, red: { type: "number", value: 255 } };
  };
  const result = await executeStandalone(d, example.inputs, example.params, () => {});
  assert.equal(result.status, "success"); assert.equal(result.outputs.red.value, 255);
  assert.equal(JSON.stringify(g()), before);
});

test("Huffman packets connect with optional ports, preserve configuration and run independently", async () => {
  const encode = add("huffman_encode"), decode = add("huffman_decode"), stats = add("hex_stats");
  connect(encode, decode, "packet", "data"); connect(decode, stats, "bytes", "data");
  assert.equal(g().edges.length, 2);
  g().setParam(encode, "mode", "table"); g().setParam(encode, "table", "a 0\nb 10\nc 11");
  g().undo(); assert.equal(node(encode).data.params.table, ""); g().redo();
  assert.equal(node(encode).data.params.table, "a 0\nb 10\nc 11");
  const d = D.getState().byId.huffman_decode;
  assert.ok(!inlineParams(d).some((p) => p.name === "weights" || p.name === "table"));
  const packet = { format: "huffman-msb", hex: "60", bitLen: 4, table: "\\x41 0\n\\x42 1" };
  const inputs = parseInputs(d.inputs, { data: { format: "json", text: JSON.stringify(packet) } });
  assert.deepEqual(inputs, { data: { type: "json", value: packet } });
  const before = JSON.stringify(g());
  api.runNodeStreamed = async (id, _nodeId, input, _params, emit) => {
    assert.equal(id, "huffman_decode"); assert.deepEqual(input, inputs);
    emit({ kind: "jobStarted", job: "huffman-standalone" });
    return { bytes: { type: "bytes", value: [65, 66, 66, 65] } };
  };
  const result = await executeStandalone(d, inputs, {}, () => {});
  assert.equal(result.status, "success"); assert.deepEqual(result.outputs.bytes.value, [65, 66, 66, 65]);
  assert.equal(JSON.stringify(g()), before);
  g().selectAll(); g().deleteSelection(); assert.equal(g().nodes.length, 0);
  g().undo(); assert.equal(g().nodes.length, 3); assert.equal(g().edges.length, 2);
});

test("geographic and Hamming parameters are validated and can be planned with typed output ports", () => {
  const plan = { notes: "decode then encode coordinates", nodes: [
    { key: "input", descriptorId: "text_input", params: { text: "u4pruydqqvj" }, position: { x: 0, y: 0 } },
    { key: "decode", descriptorId: "geo_hash_decode", params: {}, position: { x: 250, y: 0 } },
    { key: "encode", descriptorId: "geo_plus_code_encode", params: { codeLength: "15" }, position: { x: 500, y: 0 } },
  ], edges: [
    { from: { node: "input", port: "text" }, to: { node: "decode", port: "text" } },
    { from: { node: "decode", port: "text" }, to: { node: "encode", port: "text" } },
  ] };
  assert.equal(preparePlan(plan, D.getState().byId).nodes.length, 3);
  const hamming = add("hamming_code_encode"); g().setParam(hamming, "k", 11);
  assert.equal(node(hamming).data.params.k, 11); g().undo(); assert.equal(node(hamming).data.params.k, 4);
  assert.equal(D.getState().byId.geo_hash_decode.outputs.find((p) => p.name === "latitude").type, "number");
  plan.nodes[2].params.codeLength = "3";
  assert.throws(() => preparePlan(plan, D.getState().byId), /参数|选项/);
});

test("foundation nodes wire binary results, edit parameters, undo and invoke independently", async () => {
  const range = add("hex_range"), encode = add("dx_base64_encode"), decode = add("dx_base64_decode"), stats = add("hex_stats");
  connect(range, encode, "bytes", "data"); connect(encode, decode); connect(decode, stats, "bytes", "data");
  assert.equal(g().edges.length, 3);
  [range, encode, decode, stats].forEach(done);
  g().setParam(range, "start", "1");
  assert.equal(node(range).data.params.start, "1");
  assert.equal(node(stats).data.outputs, undefined);
  g().undo(); assert.equal(node(range).data.params.start, "0");
  g().redo(); assert.equal(node(range).data.params.start, "1");
  const before = JSON.stringify(g());
  api.runNodeStreamed = async (id, _nodeId, input, params, emit) => {
    assert.equal(id, "hex_range");
    assert.deepEqual(input, { data: { type: "bytes", value: [0, 255, 65] } });
    assert.deepEqual(params, { start: "1", end: "2" });
    emit({ kind: "jobStarted", job: "foundation-standalone" });
    return { bytes: { type: "bytes", value: [255, 65] } };
  };
  const d = D.getState().byId.hex_range;
  const input = parseInputs(d.inputs, { data: { format: "hex", text: "00 ff 41" } });
  const result = await executeStandalone(d, input, { start: "1", end: "2" }, () => {});
  assert.equal(result.status, "success");
  assert.deepEqual(result.outputs.bytes, { type: "bytes", value: [255, 65] });
  assert.equal(JSON.stringify(g()), before);
});

test("standalone streams typed inputs, holds foreground lock and leaves canvas untouched", async () => {
  const a = add(); done(a); const before = JSON.stringify(g());
  const wait = deferred(); const events = [];
  api.runNodeStreamed = async (did, id, inputs, params, emit) => {
    assert.deepEqual(inputs, { data: { type: "bytes", value: [0, 255] } });
    emit({ kind: "jobStarted", job: "standalone" }); emit({ kind: "nodeProgress", node: id, pct: 0.5 });
    await wait.promise; return { text: { type: "text", value: "AP8=" } };
  };
  const run = executeStandalone(D.getState().byId.base64_dict_encode, { data: { type: "bytes", value: [0, 255] } }, {}, (e) => events.push(e));
  assert.equal((await runSingleNode(a)).status, "error");
  assert.equal((await executeGraph()).status, "error");
  wait.resolve(); const result = await run;
  assert.equal(result.status, "success"); assert.equal(result.outputs.text.value, "AP8=");
  assert.equal(JSON.stringify(g()), before); assert.equal(events.length, 2);
  assert.equal(R.getState().history[0].scope, "standalone");
});

test("closing standalone before job ID cancels when known and drops late events/results", async () => {
  const pendingResult = deferred(), pendingStart = deferred(), cancelled = [], events = [];
  const controller = new AbortController(); const id = add(); done(id); const before = JSON.stringify(g());
  api.cancelJob = async (job) => { cancelled.push(job); };
  api.runNodeStreamed = async (_d, id, _i, _p, emit) => {
    await pendingStart.promise; emit({ kind: "jobStarted", job: "late-standalone" });
    emit({ kind: "nodeProgress", node: id, pct: 1 }); await pendingResult.promise;
    return { text: { type: "text", value: "stale" } };
  };
  const run = executeStandalone(D.getState().byId.base64_encode, {}, {}, (e) => events.push(e), controller.signal);
  controller.abort(); pendingStart.resolve(); await Promise.resolve(); pendingResult.resolve();
  assert.equal((await run).status, "cancelled");
  assert.deepEqual(cancelled, ["late-standalone"]); assert.equal(events.length, 0);
  assert.equal(JSON.stringify(g()), before); assert.equal(R.getState().running, false);
});

test("standalone failure event wins over successful transport and independent canvas edits", async () => {
  api.runNodeStreamed = async (_d, _id, _i, _p, emit) => { add(); emit({ kind: "jobFailed", job: "failed", error: "bad input" }); return {}; };
  const result = await executeStandalone(D.getState().byId.base64_encode, {}, {}, () => {});
  assert.equal(result.status, "error"); assert.equal(result.error, "bad input");
  const controller = new AbortController(); controller.abort();
  assert.equal((await executeStandalone(D.getState().byId.base64_encode, {}, {}, () => {}, controller.signal)).status, "cancelled");
});

test("history stores binary summaries and opaque artifact references", () => {
  assert.equal(sanitizeValue({ type: "bytes", value: [0, 255] }).type, "text");
  const ref = { type: "artifact", value: "deadbeef-aaaa-bbbb-cccc-000000000001" };
  assert.deepEqual(sanitizeValue(ref), ref);
  assert.equal(sanitizeValue({ type: "image", value: "data:image/png;base64,abc" }).type, "text");
});

test("descriptor examples keep editable typed input values", () => {
  const values = [{ type: "bytes", value: [0, 255, 65] }, { type: "text", value: " 中\n" }, { type: "number", value: 12 }, { type: "bool", value: false }, { type: "json", value: { a: 1 } }, { type: "stringList", value: ["a", ""] }];
  for (const value of values) {
    const draft = draftFromValue(value);
    assert.equal(draft.value, undefined);
    assert.deepEqual(parseInput({ name: "input", label: "输入", type: "any", required: true }, draft), value);
  }
  for (const descriptor of catalog.filter((d) => d.examples?.length)) {
    for (const example of descriptor.examples) {
      assert.deepEqual(parseInputs(descriptor.inputs, Object.fromEntries(Object.entries(example.inputs).map(([name, value]) => [name, draftFromValue(value)]))), example.inputs);
    }
  }
});

test("persisted run log entries are bounded", () => {
  const id = R.getState().startHistory({ scope: "standalone", title: "test", projectName: "", nodeCount: 1, edgeCount: 0, nodes: [] });
  R.getState().recordProgress({ kind: "log", level: "info", message: "x".repeat(10000) });
  R.getState().appendHistoryEvent(id, { time: "", level: "info", message: "y".repeat(10000) });
  assert.ok(R.getState().history[0].events.every((event) => event.message.length <= 4000));
});

beforeEach(() => {
  D.getState().setDescriptors(catalog);
  G.setState({ nodes: [], edges: [], past: [], future: [], selectedId: null, runRevision: 0, suppressHistory: false, historyGroup: null });
  R.setState({ mode: "idle", running: false, history: [], activeHistoryId: null, lastError: null });
  A.setState({ running: false, steps: [], notes: "", error: "", pendingPlan: null });
  api.resetRun = async () => {};
  api.cancelJob = async () => {};
  api.runGraph = async () => ({});
  api.runNodeStreamed = async () => ({});
  api.generateWorkflow = async () => { throw new Error("execution must never replan"); };
});

test("connecting an occupied input replaces its edge, invalidates descendants and undoes atomically", () => {
  const a = add(), b = add(), c = add("text_output");
  connect(a, c); done(c);
  const history = g().past.length;
  connect(b, c);
  assert.equal(g().edges.length, 1);
  assert.equal(g().edges[0].source, b);
  assert.equal(node(c).data.outputs, undefined);
  assert.equal(g().past.length, history + 1);
  g().undo(); assert.equal(g().edges[0].source, a);
});

test("cycles, nonexistent ports, self edges and incompatible types are rejected", () => {
  const a = add("base64_decode"), b = add("base64_decode"), img = add("image_input");
  connect(a, b); const count = g().past.length;
  connect(b, a); connect(a, a); connect(a, b, "missing"); connect(img, b, "image");
  assert.equal(g().edges.length, 1); assert.equal(g().past.length, count);
});

test("multi-delete removes incident edges in one undo and restores no stale runtime", () => {
  const a = add(), b = add("text_output"); connect(a, b); done(a); done(b);
  g().selectAll(); const count = g().past.length;
  g().deleteSelection(); assert.equal(g().nodes.length, 0); assert.equal(g().edges.length, 0);
  assert.equal(g().past.length, count + 1);
  g().undo(); assert.equal(g().nodes.length, 2); assert.equal(g().edges.length, 1);
  assert.equal(g().nodes.every((n) => n.data.status === "idle" && !n.data.outputs), true);
  g().redo(); assert.equal(g().nodes.length, 0);
});

test("drag changes are a single undo, without invalidating an executable graph revision", () => {
  const a = add(); const revision = g().runRevision, count = g().past.length;
  g().setSuppressHistory(true);
  for (const x of [10, 20, 30]) g().onNodesChange([{ type: "position", id: a, position: { x, y: 5 } }]);
  g().setSuppressHistory(false);
  assert.equal(g().past.length, count + 1); assert.equal(g().runRevision, revision);
  g().undo(); assert.deepEqual(node(a).position, { x: 0, y: 0 });
});

test("parameter changes and deleting React Flow edges/nodes invalidate downstream cached displays", () => {
  const a = add(), b = add("base64_decode"), c = add("text_output"); connect(a, b); connect(b, c);
  [a, b, c].forEach(done); g().setParam(a, "text", "changed");
  assert.equal(g().nodes.every((n) => !n.data.outputs), true);
  [a, b, c].forEach(done); g().onEdgesChange([{ type: "remove", id: g().edges[0].id }]);
  assert.ok(node(a).data.outputs); assert.equal(node(b).data.outputs, undefined); assert.equal(node(c).data.outputs, undefined);
  done(c); g().onNodesChange([{ type: "remove", id: b }]);
  assert.equal(g().edges.length, 0); assert.equal(node(c).data.outputs, undefined);
});

test("copy preserves disabled/promoted params and duplicate selects only the new node", () => {
  const a = add(); g().setDisabled(a, true); g().toggleParamInput(a, "text");
  g().paste({ nodes: [{ oldId: a, descriptorId: "text_input", label: "copied", color: "red", params: { text: "x" }, inputParams: ["text"], disabled: true, position: { x: 0, y: 0 } }], edges: [] }, 10, 20);
  assert.equal(g().nodes[1].data.disabled, true); assert.deepEqual(g().nodes[1].data.inputParams, ["text"]);
  done(a); g().duplicateNode(a);
  assert.deepEqual(g().nodes.filter((n) => n.selected).map((n) => n.id), [g().selectedId]);
  assert.equal(node(g().selectedId).data.outputs, undefined);
});

test("plan preparation preserves input whitespace and never mutates the canvas or generated plan", () => {
  add(); const original = makePlan(), before = JSON.stringify(g().nodes);
  const plan = preparePlan(original, D.getState().byId, " \nhello\t ");
  assert.equal(plan.nodes[0].params.text, " \nhello\t ");
  assert.equal(original.nodes[0].params.text, "hello"); assert.equal(JSON.stringify(g().nodes), before);
});

test("plan validation rejects unknown nodes, duplicate ids/inputs, missing inputs and invalid params", () => {
  const invalid = [
    (p) => { p.nodes[1].descriptorId = "unknown"; },
    (p) => { p.nodes[1].key = "input"; },
    (p) => { p.nodes[1].key = " "; },
    (p) => { p.edges.push(p.edges[0]); },
    (p) => { p.edges = []; },
    (p) => { p.edges[0].from.port = "missing"; },
    (p) => { p.nodes[0].params.text = 42; },
    (p) => { p.nodes[0].params.invented = true; },
    (p) => { p.nodes[0].params = []; },
    (p) => { p.nodes[0].position.x = Infinity; },
  ];
  for (const edit of invalid) { const plan = makePlan(); edit(plan); assert.throws(() => preparePlan(plan, D.getState().byId)); }
  const cycle = { notes: "", nodes: ["a", "b"].map((key) => ({ key, descriptorId: "base64_decode", params: {}, position: { x: 0, y: 0 } })), edges: [
    { from: { node: "a", port: "text" }, to: { node: "b", port: "text" } },
    { from: { node: "b", port: "text" }, to: { node: "a", port: "text" } },
  ] };
  assert.throws(() => preparePlan(cycle, D.getState().byId), /环/);
});

test("launch freezes an independent copy and does not build before confirmation is consumed", () => {
  const reviewed = { graph: makePlan(), replace: false, revision: 0 };
  A.getState().launch(reviewed); reviewed.graph.nodes[0].params.text = "changed";
  assert.equal(A.getState().pendingPlan.graph.nodes[0].params.text, "hello");
  assert.equal(g().nodes.length, 0);
});

test("confirmed plan executes exactly its new subgraph and entire append/replace is one undo", async () => {
  for (const replace of [false, true]) {
    g().loadFlow([], []); const existing = add(); const before = g().past.length;
    let sent;
    api.runGraph = async (graph, emit) => { sent = graph;
      for (const n of graph.nodes) emit({ kind: "nodeDone", node: n.id });
      return Object.fromEntries(graph.nodes.map((n) => [n.id, {}])); };
    await runAgent({ graph: makePlan(), replace, revision: g().runRevision });
    assert.equal(A.getState().error, ""); assert.equal(sent.nodes.length, 2);
    assert.equal(sent.nodes.some((n) => n.id === existing), false);
    assert.equal(g().nodes.length, replace ? 2 : 3); assert.equal(g().past.length, before + 1);
    assert.equal(g().suppressHistory, false);
    g().undo(); assert.equal(g().nodes.length, 1); assert.equal(g().nodes[0].id, existing);
  }
});

test("outdated confirmed plans never mutate the canvas", async () => {
  add(); const before = JSON.stringify(g().nodes);
  await runAgent({ graph: makePlan(), replace: true, revision: -1 });
  assert.match(A.getState().error, /画布已变化/); assert.equal(JSON.stringify(g().nodes), before);
});

test("single-node run participates in global cancellation even when stopped before jobStarted", async () => {
  const id = add(), gate = deferred(), cancelled = []; let emit;
  api.runNodeStreamed = async (_d, _n, _i, _p, onEvent) => { emit = onEvent; return gate.promise; };
  api.cancelJob = async (job) => { cancelled.push(job); };
  const task = runSingleNode(id); assert.equal(R.getState().running, true);
  const stopped = stopRun(); emit({ kind: "jobStarted", job: "late" }); emit({ kind: "nodeEntered", node: id });
  gate.resolve({ text: { type: "text", value: "late output" } });
  assert.equal((await task).status, "cancelled"); await stopped;
  assert.deepEqual(cancelled, ["late"]); assert.equal(R.getState().running, false);
  assert.equal(node(id).data.outputs, undefined); assert.equal(R.getState().history[0].status, "cancelled");
});

test("pause discards queued live reruns and late output", async () => {
  add(); let calls = 0; const gate = deferred();
  api.runGraph = async () => { calls++; return gate.promise; };
  R.getState().setMode("live"); const task = executeGraph(); await executeGraph();
  const paused = pauseRun(); gate.resolve({}); await task; await paused;
  assert.equal(calls, 1); assert.equal(R.getState().mode, "paused");
});

test("editing during execution discards stale outputs and concurrent single runs are rejected", async () => {
  const id = add(), gate = deferred();
  api.runGraph = async () => gate.promise;
  const task = executeGraph(); assert.equal((await runSingleNode(id)).status, "error");
  g().setParam(id, "text", "new"); gate.resolve({ [id]: { text: { type: "text", value: "old" } } });
  assert.equal((await task).status, "cancelled"); assert.equal(node(id).data.outputs, undefined);
});

test("node failure remains an error even if transport returns successfully", async () => {
  const id = add();
  api.runGraph = async (_graph, emit) => { emit({ kind: "nodeFailed", node: id, error: "bad input" }); return {}; };
  assert.equal((await executeGraph()).status, "error");
  assert.equal(R.getState().lastError, "bad input"); assert.equal(R.getState().history[0].status, "error");
});

test("a job-level failure cannot be reported as success without a failed node", async () => {
  add();
  api.runGraph = async (_graph, emit) => { emit({ kind: "jobFailed", job: "job", error: "graph failed" }); return {}; };
  assert.equal((await executeGraph()).status, "error");
  assert.equal(R.getState().history[0].status, "error");
});

test("running to a node executes ancestors only; history contains only that scope", async () => {
  const a = add(), b = add("text_output"), unused = add(); connect(a, b); let sent;
  api.runGraph = async (graph) => { sent = graph; return {}; };
  await executeToNode(b);
  assert.deepEqual(new Set(sent.nodes.map((n) => n.id)), new Set([a, b]));
  assert.equal(R.getState().history[0].nodes.some((n) => n.id === unused), false);
});

test("single-node execution requires cached values for connected parameters and uses them", async () => {
  const a = add(), b = add(); g().toggleParamInput(b, "text"); connect(a, b); let calls = 0, params;
  api.runNodeStreamed = async (_d, _id, _inputs, p) => { calls++; params = p; return {}; };
  assert.equal((await runSingleNode(b)).status, "error"); assert.equal(calls, 0);
  done(a); assert.equal((await runSingleNode(b)).status, "success"); assert.equal(params.text, "old");
});
