import { test } from "node:test";
import assert from "node:assert/strict";
import { agentRequest, aiRetryIssue } from "@/flow/agentRequest";
import { waitForAgentRetry } from "@/flow/agentRetry";
import { api } from "@/lib/bindings";
import { useAgentStore as A } from "@/store/agent";

const network = { code: "ai_http", message: "AI 请求超时", retryable: true };
const gate = () => { let resolve; const promise = new Promise((r) => resolve = r); return { promise, resolve }; };

test("retry policy distinguishes transport/body failures from authentication, cancellation and invalid plans", () => {
  for (const error of [network, { code: "ai_http", message: "HTTP status server error (503)" },
    { code: "ai_http", message: "HTTP 429", retryAfterMs: 2000 },
    { code: "ai_parse", message: "AI 响应解析失败：error decoding response body" },
    { code: "ai_response", message: "响应为空或截断", retryable: true }]) assert.ok(aiRetryIssue(error));
  for (const error of [{ code: "ai_http", message: "HTTP 401", retryable: true },
    { code: "ai_http", message: "HTTP 404" }, { code: "ai_http", message: "HTTP 400" },
    { code: "ai_http", message: "invalid URL", retryable: false }, { code: "cancelled", message: "取消" },
    { code: "ai_parse", message: "决策 JSON 不完整" }, { code: "ai_config", message: "缺少配置" },
    new Error("节点执行失败")]) assert.equal(aiRetryIssue(error), null);
});

test("transient model failures retry twice and late events belong only to their old attempt", async () => {
  let calls = 0, oldEmit; const messages = [], cancelledJobs = [];
  api.cancelJob = async (job) => cancelledJobs.push(job);
  const result = await agentRequest(async (emit) => {
    calls++;
    if (calls === 1) { oldEmit = emit; throw network; }
    if (calls === 2) { oldEmit({ kind: "jobStarted", job: "late-old" }); oldEmit({ kind: "log", message: "stale progress" }); throw network; }
    emit({ kind: "log", message: "current progress" }); return "recovered";
  }, new AbortController().signal, (m) => messages.push(m), { retryDelaysMs: [0, 0] });
  assert.equal(result, "recovered"); assert.equal(calls, 3);
  assert.equal(messages.filter((m) => m.includes("自动重试")).length, 2);
  assert.ok(!messages.includes("stale progress")); assert.ok(messages.includes("current progress"));
  assert.deepEqual(cancelledJobs, ["late-old"]);
});

test("legacy error decoding response body recovers without rebuilding any workflow", async () => {
  let calls = 0;
  const result = await agentRequest(async () => {
    if (!calls++) throw { code: "ai_parse", message: "AI 响应解析失败：error decoding response body" };
    return { nodes: [] };
  }, new AbortController().signal, () => {}, { retryDelaysMs: [0, 0] });
  assert.equal(calls, 2); assert.deepEqual(result, { nodes: [] });
});

test("retry exhaustion preserves the original error; permanent errors never retry", async () => {
  for (const error of [network, { code: "ai_http", message: "HTTP 401", retryable: false }]) {
    let calls = 0;
    await assert.rejects(agentRequest(async () => { calls++; throw error; }, new AbortController().signal, () => {},
      { retryDelaysMs: [0, 0] }), (actual) => actual === error);
    assert.equal(calls, error === network ? 3 : 1);
  }
});

test("cancellation interrupts Retry-After backoff before another model request", async () => {
  let calls = 0; const abort = new AbortController(), backoff = gate();
  const task = agentRequest(async () => { calls++; throw { ...network, retryAfterMs: 60000 }; }, abort.signal,
    (message) => { assert.match(message, /60 秒/); backoff.resolve(); }, { retryDelaysMs: [0, 0] });
  await backoff.promise; abort.abort(); await assert.rejects(task, /取消/); assert.equal(calls, 1);
});

test("manual retry continues the same failed request and cancellation dismisses the retry prompt", async () => {
  A.getState().start(); const abort = new AbortController(), waiting = gate(); let calls = 0;
  const off = A.subscribe((s) => { if (s.retryPrompt) waiting.resolve(); });
  const task = agentRequest(async () => { if (++calls <= 3) throw network; return "continued"; }, abort.signal, () => {},
    { retryDelaysMs: [0, 0], onRetryExhausted: (issue) => waitForAgentRetry(issue, abort.signal) });
  await waiting.promise; assert.equal(A.getState().retryPrompt.attempts, 3);
  A.getState().retryAI(); A.getState().retryAI(); assert.equal(await task, "continued");
  assert.equal(calls, 4); assert.equal(A.getState().retryPrompt, null); off();
  const pending = waitForAgentRetry({ message: "failed", attempts: 3 }, abort.signal);
  abort.abort(); await assert.rejects(pending, /取消/); assert.equal(A.getState().retryPrompt, null);
  A.getState().finish("test complete");
});
