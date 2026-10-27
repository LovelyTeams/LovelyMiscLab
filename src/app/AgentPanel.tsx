import { useEffect, useRef } from "react";
import { Check, Loader2, Sparkles, Square, X } from "lucide-react";

import { stopRun } from "@/flow/runner";
import { cn } from "@/lib/utils";
import { useAgentStore, type AgentStep } from "@/store/agent";
import { useAiStore } from "@/store/ai";
import { SolveProgressCard } from "./SolveProgressCard";

function stepColor(s: AgentStep): string {
  if (s.kind === "error" || s.ok === false) return "text-destructive";
  if (s.kind === "done") return "text-green-600";
  if (s.kind === "result") return s.ok ? "text-green-600" : "text-destructive";
  return "text-foreground";
}

/** Show the active solve alongside the graph, keeping its nodes unobscured.
 * Ordinary confirmed plans retain their compact floating report. */
export function AgentPanel() {
  const running = useAgentStore((s) => s.running);
  const cancelling = useAgentStore((s) => s.cancelling);
  const cancelled = useAgentStore((s) => s.cancelled);
  const steps = useAgentStore((s) => s.steps);
  const notes = useAgentStore((s) => s.notes);
  const error = useAgentStore((s) => s.error);
  const solveStatus = useAgentStore((s) => s.solveStatus);
  const candidates = useAgentStore((s) => s.candidates);
  const attempts = useAgentStore((s) => s.attempts);
  const solveProgress = useAgentStore((s) => s.solveProgress);
  const retryPrompt = useAgentStore((s) => s.retryPrompt);
  const reset = useAgentStore((s) => s.reset);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    listRef.current?.scrollTo({ top: listRef.current.scrollHeight });
  }, [steps.length]);

  if (!running && steps.length === 0 && !error) return null;

  return (
    <div role="region" aria-label="AI 执行报告" className={cn("z-30 flex min-h-0 max-h-[calc(100%_-_2rem)] w-[380px] flex-col rounded-lg border border-border bg-card shadow-2xl",
      solveProgress ? "relative my-4 mr-4 max-w-[45%] shrink-0" : "absolute bottom-4 right-4 max-w-[calc(100%_-_2rem)]")}>
      <div className="flex shrink-0 items-center gap-2 border-b border-border px-3 py-2">
        <span className="flex h-6 w-6 items-center justify-center rounded bg-primary/10 text-primary">
          {running ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Sparkles className="h-3.5 w-3.5" />}
        </span>
        <span className="flex-1 text-sm font-medium">
          {running ? cancelling ? "正在停止…" : retryPrompt ? "AI 请求失败 · 等待重试" : solveStatus === "working" ? "自主解题中…" : "正在执行已确认计划…" :
            solveStatus === "verified" ? "答案校验通过" : solveStatus === "candidate" ? "发现候选 · 待核验" : solveStatus === "unresolved" ? "未解决 · 尝试已停止" :
            solveStatus === "cancelled" ? "解题已停止" : error ? "计划执行 / 测试失败" : cancelled ? "计划已取消" : "计划执行结束"}
        </span>
        {running ? (
          <button
            disabled={cancelling}
            onClick={() => { useAgentStore.getState().requestCancel(); void stopRun(); }}
            className="flex items-center gap-1 rounded border border-border px-1.5 py-0.5 text-[11px] text-muted-foreground transition-colors hover:bg-destructive/10 hover:text-destructive"
          >
            <Square className="h-2.5 w-2.5" /> 停止
          </button>
        ) : (
          <button
            aria-label="关闭 AI 执行报告"
            onClick={reset}
            className="rounded p-0.5 text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
          >
            <X className="h-4 w-4" />
          </button>
        )}
      </div>

      {retryPrompt && <div role="alert" className="shrink-0 space-y-2 border-b border-border bg-amber-500/10 p-3 text-xs">
        <p className="break-words">{retryPrompt.message}</p>
        <p className="text-muted-foreground">已尝试 {retryPrompt.attempts} 次。重试将继续当前 AI 请求，保留图和执行证据；等待时间计入本次总时限。</p>
        <button disabled={cancelling} onClick={() => useAgentStore.getState().retryAI()} className="rounded border border-primary/40 bg-primary/10 px-3 py-1.5 text-primary disabled:opacity-50">重试 AI 请求</button>
      </div>}
      {solveProgress && <SolveProgressCard progress={solveProgress} />}

      {candidates.length > 0 && <div className="max-h-36 space-y-2 overflow-y-auto border-t border-border p-3 text-xs" aria-label="候选答案">
        {candidates.map((c) => <div key={c.text}>
          <code className="block select-text break-all">{c.text}</code>
          <span className="text-[10px] text-muted-foreground">第 {c.round} 轮 · {c.node}.{c.port} · {c.verified ? "与标准答案一致" : "仅格式命中"}</span>
        </div>)}
      </div>}
      {attempts.length > 0 && <details className="max-h-44 overflow-y-auto border-t border-border p-3 text-xs">
        <summary className="cursor-pointer">解题证据 · {attempts.length} 轮</summary>
        {attempts.map((a) => <details className="mt-2" key={a.round}><summary className="cursor-pointer">第 {a.round} 轮{a.error ? " · 有失败分支" : " · 已执行"}</summary>
          {a.planning && <div className="my-2 space-y-1">
            <p>观察：{a.planning.decision.observation}</p>
            <p>下一步：{a.planning.decision.nextGoal}</p>
            {(a.planning.searchHistory ?? a.planning.searches).map((s) => <p key={s.query}>查找「{s.query}」：{s.matches.length ? s.matches.map((m) => `${m.name} (${m.id})`).join("、") : "该检索词未命中"}</p>)}
            {a.planning.decision.repairs.map((r) => <p key={r.node}>修复 {r.node}：{r.problem}；修改：{r.change}；复测：{r.verify}</p>)}
          </div>}
          {!!a.changes?.length && <details><summary className="cursor-pointer">实际修改 · {a.changes.length} 项</summary>{a.changes.map((c, i) => <p key={i} className="my-1 break-all">{c}</p>)}</details>}
          {a.repairChecks?.map((c) => <p key={c.node} className="my-1">复测 {c.node}：{c.detail}</p>)}
          <p className="my-1 whitespace-pre-wrap">{a.notes}</p>
          {a.error && <p className="my-1 break-words text-destructive">{a.error}</p>}
          <pre className="max-h-40 overflow-auto whitespace-pre-wrap break-all text-[10px]">{JSON.stringify(a.evidence, null, 2)}</pre>
        </details>)}
        <button className="mt-2 rounded border border-border px-2 py-1" onClick={() => {
          const url = URL.createObjectURL(new Blob([JSON.stringify({ version: 1, status: solveStatus, candidates, attempts, notes }, null, 2)], { type: "application/json" }));
          const link = document.createElement("a"); link.href = url; link.download = "misclab-solve-report.json"; link.click();
          setTimeout(() => URL.revokeObjectURL(url), 1000);
        }}>导出解题记录</button>
      </details>}

      <div ref={listRef} className={cn("min-h-0 flex-1 space-y-0.5 overflow-y-auto px-2 py-2 text-[11px]", !solveProgress && "max-h-64")}>
        {steps.map((s, i) => (
          <div key={i} className={cn("flex items-start gap-1.5 leading-relaxed", stepColor(s))}>
            {s.kind === "done" && <Check className="mt-0.5 h-3 w-3 shrink-0" />}
            <span className="min-w-0 flex-1 break-words">
              {(["diagnose", "search", "change", "verify"] as string[]).includes(s.kind) && <span className="mr-1 rounded bg-primary/10 px-1 text-[10px] text-primary">{({ diagnose: "诊断", search: "检索", change: "修改", verify: "复测" } as Record<string, string>)[s.kind]}</span>}
              <span className="font-mono">{s.text}</span>
              {s.detail && (
                <span className="mt-0.5 block text-[10px] italic text-muted-foreground/90">
                  {s.detail}
                </span>
              )}
            </span>
          </div>
        ))}
      </div>

      {error && (
        <div className="border-t border-destructive/30 bg-destructive/10 px-3 py-1.5 text-[11px] text-destructive">
          {error}
          <button className="mt-2 block rounded border border-border px-2 py-1 text-foreground" onClick={() => {
            useAiStore.getState().setMode("repair"); useAiStore.getState().setOpen(true);
          }}>生成修复计划</button>
        </div>
      )}
      {!running && notes && !error && (
        <div className="border-t border-border px-3 py-1.5 text-[11px] text-muted-foreground">
          {notes}
        </div>
      )}
    </div>
  );
}
