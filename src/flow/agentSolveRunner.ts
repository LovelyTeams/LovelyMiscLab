import type { GeneratedGraph } from "@/lib/bindings";
import { useAgentStore as A } from "@/store/agent";
import { useDescriptorStore as D } from "@/store/descriptors";
import { useGraphStore as G } from "@/store/graph";
import { useRunStore as R } from "@/store/run";
import type { AgentRequest } from "./agentPlan";
import { applyAgentPlan, applyAgentRevision } from "./applyAgentPlan";
import { planSolveRound } from "./agentSolvePlanner";
import { waitForAgentRetry } from "./agentRetry";
import { solveChanges, solveRepairChecks } from "./agentSolveChanges";
import { checkAgentResults } from "./agentChecks";
import { acquireForeground, executeNodeSet, stopRun } from "./runner";
import { prepareSolvePlan, validateSolveOptions, solveCandidates, solveEvidence, solveFingerprint, solveGraphSummary,
  type SolveAttempt, type SolveCandidate, type SolvePlanning, type SolveProgress, type SolveResults } from "./agentSolve";

/** A bounded, explicitly approved observe → plan → execute loop. It uses the
 * same node executor/undo/history as ordinary plans, with ownership across gaps. */
export async function runSolver(reviewed: AgentRequest): Promise<void> {
  if (!reviewed.solve || A.getState().running) return;
  const options = structuredClone(reviewed.solve);
  const abort = new AbortController();
  let reason = "", applying = false, revision = reviewed.revision;
  let lease: ReturnType<typeof acquireForeground> | undefined;
  const attempts: SolveAttempt[] = [], candidates: SolveCandidate[] = [];
  const seen = new Set<string>();
  const agent = A.getState();
  agent.start();
  agent.setSolveResult("working");
  const progress = (round: number, stage: SolveProgress["stage"], message: string, extra: Partial<SolveProgress> = {}) => {
    if (!abort.signal.aborted) agent.setSolveProgress({ ...A.getState().solveProgress, round, maxRounds: options.maxRounds, stage, message, ...extra });
  };
  const nextPlan = async (round: number, previous?: GeneratedGraph) => {
    progress(round, "analyzing", round === 1 ? "分析题目和原始输入…" : "分析本轮输出，判断下一步…", { planning: undefined, plan: undefined });
    return planSolveRound(options, attempts, previous, D.getState().byId, abort.signal,
      (text) => {
        if (abort.signal.aborted) return;
        agent.pushStep({ kind: text.startsWith("查找") ? "search" : "plan", text });
        progress(round, A.getState().solveProgress?.stage ?? "analyzing", text);
      },
      (planning) => {
        if (abort.signal.aborted) return;
        progress(round, "planning", "已检索模块，正在配置参数和连线…", { planning });
        agent.pushStep({ kind: "diagnose", text: "下一步判断", detail: planning.decision.observation });
        for (const repair of planning.decision.repairs) agent.pushStep({ kind: "diagnose", text: `发现 ${repair.node} 需要修改`, detail: `${repair.problem}\n修改：${repair.change}\n复测：${repair.verify}` });
        agent.pushStep({ kind: "plan", text: "接下来需要的能力", detail: planning.decision.nextGoal });
      }, { onRetryExhausted: (issue) => waitForAgentRetry(issue, abort.signal) });
  };
  const cancel = () => { abort.abort(); if (!A.getState().cancelling) agent.requestCancel(); };
  const offGraph = G.subscribe((s) => {
    if (!applying && s.runRevision !== revision && !abort.signal.aborted) {
      reason = "画布已修改，停止自动尝试；迟到结果不会写回";
      cancel(); void stopRun();
    }
  });
  const offAgent = A.subscribe((s) => { if (s.cancelling && !abort.signal.aborted) { abort.abort(); void stopRun(); } });
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    if (revision !== G.getState().runRevision) throw new Error("画布已变化，请重新开始解题");
    validateSolveOptions(options);
    lease = acquireForeground(cancel);
    R.getState().setMode("idle");
    timer = setTimeout(() => { reason = "达到设定的总时限，已停止尝试"; cancel(); void stopRun(); }, options.maxMinutes * 60000);
    let plan: GeneratedGraph, planning: SolvePlanning | undefined;
    let previous: GeneratedGraph | undefined, ids = new Map<string, string>();
    if ("graph" in reviewed) {
      plan = prepareSolvePlan(reviewed.graph, D.getState().byId, options);
      planning = reviewed.solvePlanning;
    } else {
      const first = await nextPlan(1);
      plan = first.graph; planning = first.planning;
    }
    for (let round = 1; round <= options.maxRounds; round++) {
      if (abort.signal.aborted) break;
      const fingerprint = solveFingerprint(plan);
      if (seen.has(fingerprint)) { reason = "模型重复了已经执行的计划，停止重复尝试；请补充线索或调整范围"; break; }
      seen.add(fingerprint);
      progress(round, "building", `正在将 ${plan.nodes.length} 个节点和 ${plan.edges.length} 条连线放入画布…`, {
        planning, plan: { notes: plan.notes, nodes: plan.nodes.map((n) => ({ key: n.key, name: D.getState().byId[n.descriptorId]?.displayName ?? n.descriptorId })), edges: plan.edges.length },
      });
      agent.pushStep({ kind: "plan", text: `第 ${round}/${options.maxRounds} 轮：${plan.nodes.length} 个节点`, detail: plan.notes });
      const changes = solveChanges(previous, plan, D.getState().byId);
      for (const change of changes) agent.pushStep({ kind: "change", text: change });
      applying = true;
      try { ids = previous ? applyAgentRevision(plan, previous, ids) : applyAgentPlan(plan, reviewed.replace); }
      finally { revision = G.getState().runRevision; applying = false; }
      agent.pushStep({ kind: "run", text: `运行第 ${round} 轮并检查真实输出` });
      progress(round, "running", `正在执行第 ${round} 轮，结果直接显示在节点中`);
      const outcome = await executeNodeSet(new Set(ids.values()), `AI 解题 · 第 ${round} 轮`, lease.owner);
      if (outcome.status === "cancelled" || abort.signal.aborted) { abort.abort(); break; }
      progress(round, "checking", "检查节点输出、测试条件和候选答案…");
      const nodes = G.getState().nodes;
      const results: SolveResults = Object.fromEntries([...ids].map(([key, id]) => {
        const node = nodes.find((n) => n.id === id);
        return [key, { status: node?.data.status ?? "missing", outputs: node?.data.outputs, error: node?.data.error }];
      }));
      const checks = checkAgentResults(plan, results);
      const repairChecks = solveRepairChecks(planning, plan, results);
      for (const check of repairChecks) agent.pushStep({ kind: "verify", text: `修复复测 ${check.node}：${({ passed: "断言通过", failed: "未通过", removed: "已移除，待核验", executed: "已执行，待核验" })[check.status]}`,
        ok: check.status === "passed" ? true : check.status === "failed" ? false : undefined, detail: check.detail });
      const failures = checks.filter((s) => !s.ok);
      if (outcome.error) agent.pushStep({ kind: "error", text: outcome.error });
      for (const check of checks) agent.pushStep(check);
      if (outcome.historyId && outcome.status === "success" && failures.length)
        R.getState().finishHistory(outcome.historyId, { status: "error", error: failures.map((s) => s.text).join("；") });
      const discovered = solveCandidates(plan, results, options, round).map((c) => ({ ...c, node: ids.get(c.node)! }));
      for (const c of discovered) if (!candidates.some((old) => old.text === c.text)) candidates.push(c);
      const mismatch = options.expected && discovered.length && !discovered.some((c) => c.verified) ?
        "候选未通过本地标准答案校验，请根据证据寻找其他候选；标准答案不提供给模型。" : "";
      if (mismatch) {
        agent.pushStep({ kind: "result", ok: false, text: "本地答案校验未通过", detail: "继续尝试其他候选，标准答案保持在本地。" });
        if (outcome.historyId) R.getState().finishHistory(outcome.historyId, { status: "error", error: mismatch });
      }
      const attempt: SolveAttempt = { round, notes: plan.notes, graph: solveGraphSummary(plan), evidence: solveEvidence(plan, results),
        planning, changes, repairChecks,
        error: [outcome.error, failures.length ? failures.map((s) => s.text).join("；") : "", mismatch].filter(Boolean).join("；") || undefined };
      attempts.push(attempt); agent.recordAttempt(attempt);
      agent.setSolveResult("working", [...candidates]);
      const verdict = discovered.length ? `发现 ${discovered.length} 个符合目标格式的候选` : "本轮未发现目标答案";
      agent.pushStep({ kind: "evidence", text: verdict, detail: failures.length ? `有 ${failures.length} 项执行/断言未通过，将连同输出交回模型。` : "节点执行成功，已单独检查解题目标。" });
      if (outcome.historyId) R.getState().appendHistoryEvent(outcome.historyId, {
        time: new Date().toLocaleTimeString(), level: "info", message: `解题第 ${round} 轮：${verdict}；${failures.length} 项未通过`,
      });
      const verified = candidates.some((c) => c.verified);
      if (verified || (candidates.length && !options.expected)) {
        agent.setSolveResult(verified ? "verified" : "candidate", candidates);
        agent.finish(verified ? "实际输出与用户提供的标准答案一致。每个候选均保留来源节点和处理链。" :
          "发现符合目标格式的候选，尚未经题目平台判定。可检查来源节点，继续连接下游，或导出解题记录。每轮搭建可分别撤销。");
        return;
      }
      if (round === options.maxRounds) { reason = `已用完 ${options.maxRounds} 轮尝试，尚未确认答案；可检查已有输出并补充线索`; break; }
      agent.pushStep({ kind: "plan", text: "读取本轮证据，自动调整下一轮节点和参数…" });
      previous = plan;
      const next = await nextPlan(round + 1, previous);
      plan = next.graph; planning = next.planning;
    }
    const cancelled = abort.signal.aborted && !reason.startsWith("达到");
    agent.setSolveResult(cancelled ? "cancelled" : "unresolved", candidates);
    agent.finish(reason || "已取消自主解题，已完成轮次的证据和图保留，未完成结果不会回填。", cancelled);
  } catch (e) {
    const message = e instanceof Error ? e.message : typeof e === "object" && e && "message" in e ? String(e.message) : String(e);
    const cancelled = abort.signal.aborted && !reason.startsWith("达到");
    agent.setSolveResult(cancelled ? "cancelled" : "unresolved", candidates);
    if (!abort.signal.aborted) agent.pushStep({ kind: "error", text: `尝试中止：${message}` });
    agent.finish(reason || (cancelled ? "已取消自主解题，已完成结果保留。" : `未解决：${message.replace(/[。.!！；;\s]+$/, "")}。已保留实际输出，可调整线索后重新规划。`), cancelled);
  } finally {
    if (timer) clearTimeout(timer);
    offGraph(); offAgent(); lease?.release();
  }
}
