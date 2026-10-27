import type { SolveProgress } from "@/flow/agentSolve";

const stages: Record<SolveProgress["stage"], string> = {
  analyzing: "分析题目与证据", planning: "检索模块与规划", building: "自动搭图", running: "执行节点", checking: "检查结果与复测", finished: "本轮记录",
};

/** The active round is always visible; older evidence remains in the report. */
export function SolveProgressCard({ progress }: { progress: SolveProgress }) {
  const { planning, plan } = progress;
  return <section aria-label="当前解题规划" className="max-h-[38vh] shrink-0 space-y-2 overflow-y-auto border-b border-border bg-primary/5 px-3 py-3 text-xs">
    <div className="flex items-center justify-between gap-2 font-medium" role="status">
      <span className="rounded bg-primary/10 px-2 py-1 text-primary">{stages[progress.stage]}</span>
      <span>第 {progress.round} / {progress.maxRounds} 轮</span>
    </div>
    <p className="max-h-20 overflow-auto whitespace-pre-wrap break-words text-muted-foreground">{progress.message}</p>
    {planning && <div className="space-y-2 leading-relaxed">
      <p className="whitespace-pre-wrap break-words"><strong>当前判断：</strong>{planning.decision.observation}</p>
      <p className="whitespace-pre-wrap break-words"><strong>接下来：</strong>{planning.decision.nextGoal}</p>
      <div aria-label="模块检索结果" className="space-y-1 rounded border border-border bg-background/70 p-2">
        {planning.searches.map((s) => <p key={s.query} className="break-words">
          <span className="font-medium">查找「{s.query}」</span>：{s.matches.length ? `${s.matches.slice(0, 4).map((m) => m.name).join("、")}${s.matches.length > 4 ? `等 ${s.matches.length} 项` : ""}` : `该检索词未命中${s.suggestions?.length ? `；相关模块：${s.suggestions.slice(0, 4).map((m) => m.name).join("、")}${s.suggestions.length > 4 ? `等 ${s.suggestions.length} 项` : ""}` : ""}`}
        </p>)}
      </div>
      {planning.decision.repairs.map((r) => <p key={r.node} className="whitespace-pre-wrap break-words"><strong>修复 {r.node}：</strong>{r.problem}；{r.change}；复测：{r.verify}</p>)}
    </div>}
    {plan && <div aria-label="本轮节点计划" className="space-y-1 border-t border-border pt-2">
      <p className="font-medium">本轮计划 · {plan.nodes.length} 个节点 · {plan.edges} 条连线</p>
      <p className="whitespace-pre-wrap break-words text-muted-foreground">{plan.notes}</p>
      <ol className="grid grid-cols-2 gap-x-3 gap-y-1">{plan.nodes.map((n, i) => <li key={n.key} className="min-w-0 break-words">{i + 1}. {n.name}</li>)}</ol>
    </div>}
  </section>;
}
