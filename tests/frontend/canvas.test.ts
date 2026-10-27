import { beforeEach, describe, test } from "node:test";
import assert from "node:assert/strict";
import catalog from "../../src/lib/nodeCatalog.generated.json";
import { useDescriptorStore as D } from "@/store/descriptors";
import { useGraphStore as G } from "@/store/graph";
import { canvasShortcut } from "@/flow/canvasShortcuts";
import { copySelection, cutSelection, duplicateSelection, pasteSelection, selectedSubgraph } from "@/flow/graphClipboard";

const g = () => G.getState();
const node = (id: string) => g().nodes.find((n) => n.id === id)!;
const add = (id = "text_input", x = 0, y = 0) => g().addNode(D.getState().byId[id], { x, y });
const conn = (source: string, target: string) => ({ source, target, sourceHandle: "text", targetHandle: "text" });
const done = (id: string) => g().updateRuntime(id, { status: "done", outputs: { text: { type: "text", value: "cached" } } });

describe("canvas editing and Windows shortcuts", () => {
  beforeEach(() => {
    D.getState().setDescriptors(catalog);
    G.setState({ nodes: [], edges: [], past: [], future: [], selectedId: null, runRevision: 0, suppressHistory: false, historyGroup: null });
  });

  test("one field editing session is one undo; refocusing creates the next undo", () => {
    const a = add(), b = add("text_output"); g().onConnect(conn(a, b)); done(b);
    const key = `param:${a}:text`, count = g().past.length;
    g().beginHistoryGroup(key);
    for (const value of ["h", "he", "hello"]) g().setParam(a, "text", value);
    g().endHistoryGroup(key);
    assert.equal(g().past.length, count + 1); assert.equal(node(b).data.outputs, undefined);
    g().undo(); assert.equal(node(a).data.params.text, "");
    g().redo(); assert.equal(node(a).data.params.text, "hello");
    g().beginHistoryGroup(key); g().setParam(a, "text", "hello world"); g().endHistoryGroup(key);
    g().undo(); assert.equal(node(a).data.params.text, "hello");
  });

  test("invalid or unchanged values do not poison params, revision, redo or history", () => {
    const a = add("qr_encode"); g().setParam(a, "scale", 9); g().undo();
    const previous = JSON.stringify(g());
    for (const value of [NaN, Infinity, -Infinity, node(a).data.params.scale]) g().setParam(a, "scale", value);
    g().setParam("missing", "scale", 5); g().renameNode(a, node(a).data.label);
    assert.equal(JSON.stringify(g()), previous);
  });

  test("other graph changes split an edit group and clear redo", () => {
    const a = add(), key = `param:${a}:text`;
    g().beginHistoryGroup(key); g().setParam(a, "text", "first");
    const b = add(); g().setParam(a, "text", "second");
    g().undo(); assert.equal(node(a).data.params.text, "first"); assert.ok(node(b));
    g().undo(); assert.equal(g().nodes.length, 1);
    g().setParam(a, "text", "new branch"); assert.equal(g().canRedo(), false);
  });

  test("held direction keys move the selected group and undo once without dropping results", () => {
    const a = add("text_input", 10, 20), b = add("text_output", 200, 50);
    g().selectAll(); done(a); const revision = g().runRevision, count = g().past.length;
    g().beginHistoryGroup("moveSelection");
    for (let i = 0; i < 8; i++) g().moveSelection(10, -1);
    g().endHistoryGroup("moveSelection");
    assert.deepEqual(node(a).position, { x: 90, y: 12 }); assert.deepEqual(node(b).position, { x: 280, y: 42 });
    assert.equal(g().runRevision, revision); assert.equal(node(a).data.status, "done");
    assert.equal(g().past.length, count + 1);
    g().undo(); assert.deepEqual(node(a).position, { x: 10, y: 20 }); assert.deepEqual(node(b).position, { x: 200, y: 50 });
    g().redo(); assert.deepEqual(node(a).position, { x: 90, y: 12 });
  });

  test("the final unchanged drag event does not create an extra undo", () => {
    const a = add(), count = g().past.length;
    g().setSuppressHistory(true);
    g().onNodesChange([{ type: "position", id: a, position: { x: 50, y: 30 }, dragging: true }]);
    g().setSuppressHistory(false);
    g().onNodesChange([{ type: "position", id: a, position: { x: 50, y: 30 }, dragging: false }]);
    assert.equal(g().past.length, count + 1);
    g().undo(); assert.deepEqual(node(a).position, { x: 0, y: 0 });
  });

  test("mixed node and edge deletion restores all attached edges in one undo", () => {
    const a = add(), b = add("base64_decode"), c = add("text_output"), d = add("text_output");
    g().onConnect(conn(a, b)); g().onConnect(conn(b, c)); g().onConnect(conn(a, d));
    const detached = g().edges.find((e) => e.target === d)!;
    g().selectNode(b); g().onEdgesChange([{ type: "select", id: detached.id, selected: true }]);
    const count = g().past.length; g().deleteSelection();
    assert.equal(g().nodes.length, 3); assert.equal(g().edges.length, 0);
    assert.equal(g().past.length, count + 1); g().undo();
    assert.equal(g().nodes.length, 4); assert.equal(g().edges.length, 3); assert.equal(g().selectedId, b);
    g().redo(); assert.equal(g().nodes.length, 3); assert.equal(g().edges.length, 0);
  });

  test("copy/paste, cut and duplicate preserve the selected subgraph, not external edges", () => {
    const a = add(), b = add("base64_decode"), c = add("text_output");
    g().onConnect(conn(a, b)); g().onConnect(conn(b, c));
    g().setParam(a, "text", "SGVsbG8="); g().setDisabled(b, true); g().toggleParamInput(b, "alphabet");
    g().selectNode(a); g().onNodesChange([{ type: "select", id: b, selected: true }]);
    const clip = selectedSubgraph()!; assert.equal(clip.nodes.length, 2); assert.equal(clip.edges.length, 1);
    assert.equal(copySelection(), true); g().setParam(a, "text", "changed");
    const count = g().past.length; pasteSelection({ x: 500, y: 600 });
    const pasted = g().nodes.filter((n) => n.selected);
    assert.equal(pasted.length, 2); assert.equal(pasted[0].data.params.text, "SGVsbG8=");
    assert.equal(pasted[1].data.disabled, true); assert.deepEqual(pasted[1].data.inputParams, ["alphabet"]);
    assert.equal(g().edges.length, 3); assert.equal(g().past.length, count + 1);
    assert.deepEqual(pasted[0].position, { x: 500, y: 600 });
    g().undo(); assert.equal(g().nodes.length, 3); g().redo();
    cutSelection(); assert.equal(g().nodes.length, 3); g().undo(); assert.equal(g().nodes.length, 5);
    duplicateSelection(); assert.equal(g().nodes.length, 7); g().undo(); assert.equal(g().nodes.length, 5);
  });

  test("all selection includes edges; clearing selection and empty deletion preserve undo", () => {
    const a = add(), b = add("text_output"); g().onConnect(conn(a, b));
    const count = g().past.length; g().selectAll(); assert.ok(g().edges.every((e) => e.selected));
    g().deselectAll(); g().deleteSelection(); assert.equal(g().past.length, count);
    assert.equal(g().selectedId, null); assert.ok(g().nodes.every((n) => !n.selected));
  });

  test("additive box selection preserves the old nodes and edges without adding undo entries", () => {
    const a = add(), b = add("text_output"), c = add("qr_encode"); g().onConnect(conn(a, b));
    g().selectAll(); const before = g().past.length, edge = g().edges[0].id;
    g().selectNode(c); g().addToSelection([a], [edge]);
    assert.deepEqual(g().nodes.filter((n) => n.selected).map((n) => n.id), [a, c]);
    assert.equal(g().edges[0].selected, true); assert.equal(g().past.length, before);
  });

  test("reconnecting replaces an occupied input atomically, invalidates both targets and undoes", () => {
    const a = add(), b = add("base64_decode"), c = add("text_output");
    g().onConnect(conn(a, b)); g().onConnect(conn(b, c)); done(b); done(c);
    const edge = g().edges[0], original = structuredClone(g().edges), count = g().past.length;
    g().reconnectEdge(edge.id, conn(a, c));
    assert.equal(g().edges.length, 1); assert.equal(g().edges[0].target, c);
    assert.equal(node(b).data.outputs, undefined); assert.equal(node(c).data.outputs, undefined);
    assert.equal(g().past.length, count + 1); g().undo(); assert.deepEqual(g().edges, original);
  });

  test("invalid reconnects and cycle attempts preserve the original edge and history", () => {
    const a = add(), b = add("base64_decode"), c = add("base64_decode");
    g().onConnect(conn(a, b)); g().onConnect(conn(b, c)); const edge = g().edges[0], before = JSON.stringify(g());
    g().reconnectEdge(edge.id, conn(c, b)); g().reconnectEdge(edge.id, conn(a, a));
    g().reconnectEdge(edge.id, { ...conn(a, b), targetHandle: "missing" });
    g().reconnectEdge(edge.id, conn(a, b)); assert.equal(JSON.stringify(g()), before);
  });

  test("Windows shortcuts cover editing, navigation, redo variants and IME boundaries", () => {
    const event = (key: string, extra = {}) => ({ key, ctrlKey: false, metaKey: false, altKey: false, shiftKey: false, isComposing: false, ...extra });
    for (const [key, action] of Object.entries({ a: "selectAll", c: "copy", v: "paste", x: "cut", d: "duplicate", z: "undo", y: "redo", "0": "actualSize", "=": "zoomIn", "-": "zoomOut" })) {
      assert.equal(canvasShortcut(event(key, { ctrlKey: true })), action);
      assert.equal(canvasShortcut(event(key, { metaKey: true })), action);
    }
    assert.equal(canvasShortcut(event("Z", { ctrlKey: true, shiftKey: true })), "redo");
    for (const [key, action] of Object.entries({ Delete: "delete", Backspace: "delete", Escape: "deselect", F2: "rename", Home: "fit", ArrowRight: "right" })) assert.equal(canvasShortcut(event(key)), action);
    assert.equal(canvasShortcut(event("Delete", { isComposing: true })), null);
    assert.equal(canvasShortcut(event("a", { ctrlKey: true, altKey: true })), null);
    assert.equal(canvasShortcut(event("c", { ctrlKey: true, shiftKey: true })), null);
  });
});
