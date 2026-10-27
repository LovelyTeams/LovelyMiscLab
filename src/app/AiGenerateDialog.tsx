import { useEffect, useRef, useState } from "react";
import { Bot, Check, Loader2, RefreshCw, Sparkles, X } from "lucide-react";
import { open as pickFiles } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import { cn } from "@/lib/utils";
import { preparePlan, withExpectedText, type ReviewedPlan } from "@/flow/agentPlan";
import { agentRequest } from "@/flow/agentRequest";
import { validateSolveOptions, type SolveOptions } from "@/flow/agentSolve";
import { WidgetRenderer } from "@/flow/WidgetRenderer";
import { buildGraph } from "@/flow/runner";
import { useAgentStore } from "@/store/agent";
import { useAiStore } from "@/store/ai";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";
import { useRunStore } from "@/store/run";
import { useViewStore } from "@/store/view";

const MODES = [
  { mode: "solve" as const, label: "自主解题", icon: Bot },
  { mode: "generate" as const, label: "规划流程", icon: Sparkles },
  { mode: "explain" as const, label: "解释画布", icon: Bot },
  { mode: "repair" as const, label: "规划修复", icon: RefreshCw },
];
const EXAMPLES = ["对输入文本计算 SHA256", "循环解码 Base64，直到出现 flag", "生成一个二维码，内容是 flag{hello}"];

export function AiGenerateDialog() {
  const open = useAiStore((s) => s.open);
  const descriptors = useDescriptorStore((s) => s.byId);
  const revision = useGraphStore((s) => s.runRevision);
  const running = useRunStore((s) => s.running);
  const agentRunning = useAgentStore((s) => s.running || !!s.pendingPlan);
  const mode = useAiStore((s) => s.mode);
  const setMode = useAiStore((s) => s.setMode);
  const [prompt, setPrompt] = useState("");
  const [data, setData] = useState("");
  const [expected, setExpected] = useState("");
  const [files, setFiles] = useState("");
  const [prefix, setPrefix] = useState("flag{");
  const [maxRounds, setMaxRounds] = useState(5);
  const [maxMinutes, setMaxMinutes] = useState(8);
  const [progress, setProgress] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [answer, setAnswer] = useState("");
  const [plan, setPlan] = useState<ReviewedPlan | null>(null);
  const [replace, setReplace] = useState(false);
  const request = useRef(0);
  const controller = useRef<AbortController | null>(null);
  useEffect(() => {
    if (!open) { request.current++; controller.current?.abort(); setLoading(false); setPlan(null); }
  }, [open]);
  useEffect(() => () => { request.current++; controller.current?.abort(); }, []);
  if (!open) return null;

  const close = () => {
    request.current++;
    controller.current?.abort();
    setLoading(false);
    setPlan(null);
    useAiStore.getState().setOpen(false);
  };
  const invalidate = () => { request.current++; controller.current?.abort(); setLoading(false); setPlan(null); setError(""); setAnswer(""); setProgress(""); };
  const run = async () => {
    if (loading || agentRunning || running) return;
    if (!inTauri) { setError("请在桌面应用中配置 AI 模型后生成计划。浏览器预览不调用模型。"); return; }
    if ((mode === "generate" || mode === "solve") && !prompt.trim()) return;
    if (mode === "solve") {
      try {
        const solve: SolveOptions = { task: prompt.trim(), data, expected, prefix,
          files: [...new Set(files.split(/\r?\n/).map((f) => f.trim()).filter(Boolean))], maxRounds, maxMinutes };
        validateSolveOptions(solve);
        if (!useAgentStore.getState().launchSolve({ solve, replace, revision: useGraphStore.getState().runRevision }))
          throw new Error("已有 AI 任务，请先停止当前任务");
        useRunStore.getState().setMode("idle");
        useViewStore.getState().setView("canvas");
        close();
      } catch (e) { setError(e instanceof Error ? e.message : String(e)); }
      return;
    }
    const ticket = ++request.current;
    const abort = new AbortController();
    controller.current = abort;
    const initialRevision = useGraphStore.getState().runRevision;
    setLoading(true); setPlan(null); setError(""); setAnswer("");
    setProgress("正在选择节点、配置参数和连线…");
    const report = (message: string) => { if (ticket === request.current) setProgress(message); };
    try {
      if (mode === "explain") {
        const result = await agentRequest((emit) => api.explainWorkflow(buildGraph(), prompt.trim(), emit), abort.signal, report);
        if (ticket === request.current) setAnswer(result.text);
      } else {
        const instructions = [
          prompt.trim(),
          "只生成待审核的计划；notes 说明步骤、预期结果和执行时会读写的文件或访问的地址。不要执行任何步骤。",
          mode === "repair" ? `根据当前流程提出修复计划。最近错误：${useRunStore.getState().lastError || "无"}\n当前流程：${JSON.stringify(buildGraph())}` : "",
          data.length ? `用户提供了 ${data.length} 字待处理数据。使用 text_input，text 留空，程序将在计划中原样填入。数据开头（仅供识别，不要复述）：${data.slice(0, 200)}` : "",
        ].filter(Boolean).join("\n\n");
        const generated = await agentRequest((emit) => api.generateWorkflow(instructions, emit), abort.signal, report);
        if (ticket === request.current)
          setPlan({ graph: withExpectedText(preparePlan(generated, descriptors, data), expected, descriptors), replace, revision: initialRevision });
      }
    } catch (e) {
      if (ticket === request.current) setError(e instanceof Error ? e.message : typeof e === "object" && e && "message" in e ? String(e.message) : String(e));
    } finally { if (ticket === request.current) setLoading(false); }
  };
  const confirmPlan = () => {
    if (!plan || loading || running || agentRunning) return;
    if (plan.revision !== useGraphStore.getState().runRevision) { setError("画布已修改，请重新生成计划后确认"); return; }
    try {
      const graph = preparePlan(plan.graph, useDescriptorStore.getState().byId);
      useRunStore.getState().setMode("idle");
      useAgentStore.getState().launch({ ...plan, graph, replace });
    } catch (e) { setError(e instanceof Error ? e.message : String(e)); return; }
    useViewStore.getState().setView("canvas");
    close();
  };
  const stale = plan && plan.revision !== revision;
  return (
    <div className="fixed inset-0 z-[75] flex items-center justify-center bg-black/50 p-4" onClick={close}>
      <div role="dialog" aria-modal="true" aria-labelledby="agent-dialog-title"
        className="flex max-h-[90vh] w-[760px] max-w-[95vw] flex-col rounded-xl border border-border bg-card shadow-2xl"
        onClick={(e) => e.stopPropagation()} onKeyDown={(e) => {
          if (e.key === "Escape") close();
          // Solve starts within the displayed scope; other modes generate a preview.
          if ((e.ctrlKey || e.metaKey) && e.key === "Enter") { e.preventDefault(); void run(); }
        }}>
        <div className="flex items-center gap-3 border-b border-border p-4">
          <Sparkles className="h-6 w-6 text-primary" />
          <div className="flex-1"><h2 id="agent-dialog-title" className="font-semibold">AI 工作流助手</h2>
            <p className="mt-1 text-xs text-muted-foreground">{mode === "solve" ? "点一次开始，规划、检索、搭图和复测直接显示在画布上，随时可停止。" : "生成计划并审核确认后，自动选节点、配参数、搭图和测试。"}</p></div>
          <button aria-label="关闭 AI 助手" onClick={close}><X className="h-5 w-5" /></button>
        </div>
        <div className="flex gap-2 border-b border-border px-4 py-3">
          {MODES.map((m) => <button key={m.mode} disabled={loading} onClick={() => { invalidate(); setMode(m.mode); }}
            className={cn("flex items-center gap-1.5 rounded-md border px-3 py-2 text-xs disabled:opacity-50",
              mode === m.mode ? "border-primary bg-primary/10 text-primary" : "border-border hover:bg-accent")}>
            <m.icon className="h-3.5 w-3.5" />{m.label}</button>)}
        </div>
        <div className="min-h-0 flex-1 space-y-3 overflow-y-auto p-4">
          <label className="block text-xs font-medium">任务描述
            <textarea autoFocus value={prompt} disabled={loading} onChange={(e) => { invalidate(); setPrompt(e.target.value); }} rows={3}
              placeholder={mode === "generate" || mode === "solve" ? "描述目标、题目提示和已知线索；未知的步骤交给 Agent 分析" : "可选：描述关注点或希望如何修复"}
              className="mt-1 w-full rounded-md border border-input bg-background p-3 text-sm" />
          </label>
          {mode !== "explain" && <label className="block text-xs font-medium">待处理数据（可选，保留原始空白）
            <textarea value={data} disabled={loading} onChange={(e) => { invalidate(); setData(e.target.value); }} rows={3}
              placeholder={mode === "solve" ? "原始密文、题目文本；完整输入交给节点处理，模型查看有界摘要" : "密文或原始文本；完整内容直接填入计划中的第一个文本输入节点，模型只看前 200 字"}
              className="mt-1 w-full rounded-md border border-input bg-background p-3 font-mono text-xs" />
          </label>}
          {mode === "solve" && <div className="space-y-3 rounded-lg border border-border p-3">
            <div className="flex items-center justify-between text-xs font-medium">
              <span>题目文件：压缩包、图片、音频、流量或其他文件</span>
              <button disabled={loading} className="rounded border border-input px-2 py-1" onClick={async () => {
                try {
                  const selected = await pickFiles({ multiple: true, directory: false });
                  if (selected) { invalidate(); setFiles((current) => [...new Set([...current.split(/\r?\n/).filter(Boolean), ...[selected].flat()])].join("\n")); }
                } catch (e) { setError(String(e)); }
              }}>选择题目文件</button>
            </div>
            <textarea aria-label="题目文件路径" rows={2} disabled={loading} value={files}
              onChange={(e) => { invalidate(); setFiles(e.target.value); }} placeholder="也可以粘贴完整本地路径，每行一个，最多 8 个"
              className="w-full rounded border border-input bg-background p-2 font-mono text-xs" />
            <div className="grid grid-cols-3 gap-3 text-xs">
              <label>答案前缀<input value={prefix} disabled={loading} maxLength={64} onChange={(e) => { invalidate(); setPrefix(e.target.value); }}
                className="mt-1 w-full rounded border border-input bg-background p-2 font-mono" /></label>
              <label>最多尝试轮数<input type="number" min={1} max={8} value={maxRounds} disabled={loading} onChange={(e) => { invalidate(); setMaxRounds(Number(e.target.value)); }}
                className="mt-1 w-full rounded border border-input bg-background p-2" /></label>
              <label>总时限（分钟）<input type="number" min={1} max={15} value={maxMinutes} disabled={loading} onChange={(e) => { invalidate(); setMaxMinutes(Number(e.target.value)); }}
                className="mt-1 w-full rounded border border-input bg-background p-2" /></label>
            </div>
            <p className="text-[11px] text-muted-foreground">适用现有文本、密码、文件、压缩包、图像和音频模块。模型接收题目描述、文件路径及运行摘要；大文件内容由本机节点处理。以答案前缀与右花括号识别候选，格式命中需人工核验。</p>
            <label className="flex items-center gap-2 text-xs"><input type="checkbox" checked={replace} onChange={(e) => setReplace(e.target.checked)} />替换当前画布（默认追加到旁边）</label>
            <p className="rounded bg-primary/5 p-2 text-xs leading-relaxed">点击“开始解题”后，自动使用本次文本、选定文件和内置本地处理模块，最多 {maxRounds} 轮、{maxMinutes} 分钟，每轮最多 20 个节点。计划生成后直接搭图执行，无需再次确认；每轮修改可撤销。</p>
          </div>}
          {mode !== "explain" && <label className="block text-xs font-medium">{mode === "solve" ? "标准答案（可选，仅本地验收，不交给模型）" : "预期文本（可选，精确匹配最终文本输出）"}
            <textarea value={expected} disabled={loading} maxLength={4096} rows={2}
              onChange={(e) => { invalidate(); setExpected(e.target.value); }} placeholder={mode === "solve" ? "解题时通常留空；有标准答案时可检验真实输出" : "例如 Hello；保留空白，留空时只检查执行状态"}
              className="mt-1 w-full rounded-md border border-input bg-background p-2 font-mono text-xs" />
          </label>}
          {mode === "generate" && <div className="flex flex-wrap gap-2">{EXAMPLES.map((text) =>
            <button key={text} disabled={loading} onClick={() => { invalidate(); setPrompt(text); }}
              className="rounded-full bg-secondary px-3 py-1 text-[11px] text-muted-foreground">{text}</button>)}</div>}
          {plan && <section aria-label="待确认计划" className="space-y-3 rounded-lg border border-primary/40 bg-primary/5 p-3">
            <div className="flex items-center gap-2 font-medium"><Check className="h-4 w-4 text-primary" />待确认计划
              <span className="ml-auto text-xs text-muted-foreground">{plan.graph.nodes.length} 个节点 · {plan.graph.edges.length} 条连线</span></div>
            <p className="whitespace-pre-wrap text-sm">{plan.graph.notes || "按以下步骤处理数据。"}</p>
            <ol className="space-y-2">{plan.graph.nodes.map((node, index) => <li key={node.key} className="rounded border border-border bg-background p-2 text-xs">
              <div className="font-medium">{index + 1}. {descriptors[node.descriptorId]?.displayName} <span className="text-muted-foreground">({node.key})</span></div>
              <details className="mt-1"><summary className="cursor-pointer text-muted-foreground">查看 / 调整参数</summary>
                <div className="mt-2 space-y-2">{descriptors[node.descriptorId]?.params.map((spec) => <div key={spec.name}>
                  <div className="mb-1 text-muted-foreground">{spec.label}</div>
                  <WidgetRenderer spec={spec} value={node.params[spec.name]} onChange={(value) => {
                    setError(""); setPlan((current) => current && ({ ...current, graph: { ...current.graph,
                      nodes: current.graph.nodes.map((n) => n.key === node.key ? { ...n, params: { ...n.params, [spec.name]: value } } : n) } }));
                  }} />
                </div>)}</div></details>
            </li>)}</ol>
            <div className="space-y-1 text-xs" aria-label="自动测试条件">
              <p>自动测试：检查每个节点执行状态和结果返回；{plan.graph.checks?.length ?? 0} 项结果断言。</p>
              {plan.graph.checks?.map((check, i) => <div key={i} className="flex items-center gap-2 rounded bg-background p-2">
                <span className="min-w-0 flex-1 whitespace-pre-wrap break-all font-mono">{check.node}.{check.port} {check.kind === "equals" ? "等于" : check.kind === "contains" ? "包含" : "非空"}{check.kind !== "nonEmpty" && ` ${JSON.stringify(check.expected)}`}</span>
                <button aria-label={`移除测试条件 ${i + 1}`} onClick={() => setPlan({ ...plan, graph: { ...plan.graph, checks: plan.graph.checks?.filter((_, index) => index !== i) } })}><X className="h-3 w-3" /></button>
              </div>)}
            </div>
            <details><summary className="cursor-pointer text-xs">查看连线</summary>
              <ul className="mt-2 space-y-1 font-mono text-[11px]">{plan.graph.edges.map((edge, i) =>
                <li key={i}>{edge.from.node}.{edge.from.port} → {edge.to.node}.{edge.to.port}</li>)}</ul></details>
            <label className="flex items-center gap-2 text-xs"><input type="checkbox" checked={replace} onChange={(e) => setReplace(e.target.checked)} />替换当前画布（默认追加到旁边）</label>
            <p className="text-xs text-muted-foreground">确认后执行上述节点；文件读写、网络请求等行为由所列节点及参数决定。</p>
          </section>}
          {stale && <p role="alert" className="text-xs text-destructive">画布已修改，该计划已过期。请重新生成并检查。</p>}
          {error && <p role="alert" className="whitespace-pre-wrap rounded bg-destructive/10 p-3 text-xs text-destructive">{error}</p>}
          {loading && <p role="status" className="text-xs text-muted-foreground">{progress}</p>}
          {answer && <pre className="whitespace-pre-wrap rounded border border-border p-3 text-sm">{answer}</pre>}
        </div>
        <div className="flex flex-wrap items-center justify-between gap-2 border-t border-border p-4">
          <span className="text-[11px] text-muted-foreground">Ctrl + Enter {mode === "solve" ? "开始解题" : "生成预览"}</span>
          <div className="flex gap-2">
            {loading && <Button variant="outline" size="sm" onClick={() => { invalidate(); setAnswer("已取消规划，可修改任务后重新生成。"); }}>停止规划</Button>}
            <Button variant="outline" size="sm" onClick={close}>取消</Button>
            <Button variant={plan ? "outline" : "default"} size="sm" onClick={() => void run()}
              disabled={loading || agentRunning || running || ((mode === "generate" || mode === "solve") && !prompt.trim())}>
              {loading && <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" />}
              {loading ? "正在规划…" : mode === "solve" ? "开始解题" : mode === "explain" ? "解释流程" : plan ? "重新生成计划" : "生成计划"}
            </Button>
            {plan && <Button size="sm" disabled={!!stale || loading || running || agentRunning} onClick={confirmPlan}>确认搭建并运行</Button>}
          </div>
        </div>
      </div>
    </div>
  );
}
