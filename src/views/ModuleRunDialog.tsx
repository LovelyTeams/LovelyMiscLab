import { useEffect, useRef, useState } from "react";
import { Play, RotateCcw, Square, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ProgressBar } from "@/components/ui/progress";
import { categoryLabel } from "@/lib/catalog";
import { draftFromValue, parseInputs, type InputDraft } from "@/lib/typedInputs";
import type { NodeDescriptor, PortValue, ProgressMsg } from "@/lib/types";
import { useRunStore } from "@/store/run";
import { WidgetRenderer } from "@/flow/WidgetRenderer";
import { PortInputEditor } from "@/flow/PortInputEditor";
import { OutputValue } from "@/flow/portValue";
import { executeStandalone, sanitizeOutputs } from "@/flow/runner";

export function ModuleRunDialog({ descriptor, onClose }: { descriptor: NodeDescriptor; onClose: () => void }) {
  const defaults = () => Object.fromEntries(descriptor.params.map((p) => [p.name, p.default]));
  const [inputs, setInputs] = useState<Record<string, InputDraft>>({});
  const [params, setParams] = useState<Record<string, unknown>>(defaults);
  const [outputs, setOutputs] = useState<Record<string, PortValue>>();
  const [error, setError] = useState("");
  const [status, setStatus] = useState("待运行");
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState(0);
  const [elapsed, setElapsed] = useState(0);
  const [tab, setTab] = useState<"result" | "logs" | "history">("result");
  const [logs, setLogs] = useState<string[]>([]);
  const [history, setHistory] = useState<{ time: string; status: string; outputs?: Record<string, PortValue> }[]>([]);
  const controller = useRef<AbortController | null>(null);
  const mounted = useRef(true);
  const busy = useRunStore((s) => s.running);

  useEffect(() => {
    mounted.current = true;
    return () => { mounted.current = false; controller.current?.abort(); };
  }, []);
  const close = () => { controller.current?.abort(); onClose(); };
  const onEvent = (event: ProgressMsg) => {
    if (!mounted.current) return;
    if (event.kind === "nodeProgress") setProgress(event.pct);
    if (event.kind === "log") setLogs((l) => [...l, event.level + ": " + event.message.slice(0, 4000)].slice(-300));
    if (event.kind === "nodeFailed" || event.kind === "jobFailed") setError(event.error);
  };
  const run = async () => {
    if (controller.current) return;
    let values: Record<string, PortValue>;
    try { values = parseInputs(descriptor.inputs, inputs); }
    catch (e) { setError(e instanceof Error ? e.message : String(e)); return; }
    const abort = new AbortController();
    controller.current = abort;
    setRunning(true); setError(""); setOutputs(undefined); setLogs([]); setElapsed(0); setProgress(0); setStatus("运行中"); setTab("result");
    const result = await executeStandalone(descriptor, values, { ...params }, onEvent, abort.signal);
    if (!mounted.current || controller.current !== abort) return;
    const label = result.status === "success" ? "执行成功" : result.status === "cancelled" ? "已取消" : "执行失败";
    setStatus(label); setRunning(false); setElapsed(result.elapsed ?? 0);
    setError(result.error ?? ""); setOutputs(result.outputs);
    setHistory((h) => [{ time: new Date().toLocaleTimeString(), status: label, outputs: sanitizeOutputs(result.outputs) }, ...h].slice(0, 20));
    controller.current = null;
  };
  return (
    <div className="fixed inset-0 z-[70] flex items-center justify-center bg-black/50 p-4" onClick={close}>
      <div role="dialog" aria-modal="true" aria-label={descriptor.displayName + " · 单独调用"}
        className="flex max-h-[90vh] w-[1080px] max-w-[96vw] flex-col overflow-hidden rounded-xl border border-border bg-card shadow-2xl"
        onClick={(e) => e.stopPropagation()} onKeyDown={(e) => { if (e.key === "Escape") { e.stopPropagation(); close(); } }}>
        <div className="flex items-start gap-3 border-b border-border px-5 py-4">
          <div className="min-w-0 flex-1"><div className="text-lg font-semibold">{descriptor.displayName}</div>
            <p className="text-xs text-muted-foreground">{categoryLabel(descriptor)} · 单独调用</p>
            <p className="mt-2 text-xs">{descriptor.description}</p></div>
          <button aria-label="关闭" onClick={close}><X className="h-5 w-5" /></button>
        </div>
        <div className="grid min-h-0 flex-1 grid-cols-1 md:grid-cols-2">
          <div className="space-y-5 overflow-auto border-r border-border p-5">
            {!!descriptor.examples?.length && <div className="flex flex-wrap gap-2">{descriptor.examples.map((example, i) =>
              <button key={i} disabled={running} className="rounded bg-secondary px-2 py-1 text-xs" onClick={() => {
                setInputs(Object.fromEntries(Object.entries(example.inputs).map(([name, value]) => [name, draftFromValue(value)])));
                setParams({ ...defaults(), ...example.params }); setError(""); setOutputs(undefined);
              }}>示例：{example.label}</button>)}</div>}
            {descriptor.inputs.map((port) => <PortInputEditor key={port.name} port={port} draft={inputs[port.name]} disabled={running}
              onChange={(value) => setInputs((prev) => ({ ...prev, [port.name]: value }))} />)}
            <fieldset disabled={running} className="space-y-3">
              {!!descriptor.params.length && <legend className="mb-2 text-sm font-semibold">参数配置</legend>}
              {descriptor.params.map((p) => <div key={p.name}><label className="mb-1 block text-xs text-muted-foreground">{p.label}</label>
                <WidgetRenderer spec={p} value={params[p.name]} onChange={(value) => setParams((prev) => ({ ...prev, [p.name]: value }))} />
                {p.description && <p className="mt-1 text-[10px] text-muted-foreground">{p.description}</p>}</div>)}
            </fieldset>
          </div>
          <div className="flex min-h-0 flex-col p-5">
            <div className="flex gap-3 border-b border-border pb-2 text-xs">
              {(["result", "logs", "history"] as const).map((t, i) => <button key={t} onClick={() => setTab(t)} className={tab === t ? "text-primary" : "text-muted-foreground"}>{["结果", "日志", "历史摘要"][i]}</button>)}
            </div>
            <div className="min-h-0 flex-1 space-y-3 overflow-auto py-3">
              {tab === "result" && <>
                {error && <p role="alert" className="whitespace-pre-wrap rounded bg-destructive/10 p-3 text-xs text-destructive">{error}</p>}
                {!outputs && !error && <p className="py-10 text-center text-xs text-muted-foreground">{running ? "正在执行…" : "运行后在这里查看结果"}</p>}
                {outputs && [...descriptor.outputs.map((o) => o.name), ...Object.keys(outputs).filter((k) => !descriptor.outputs.some((o) => o.name === k))]
                  .filter((k) => outputs[k]).map((key) => <section key={key}><div className="mb-1 text-xs font-medium">{descriptor.outputs.find((o) => o.name === key)?.label ?? key}</div>
                    <OutputValue value={outputs[key]} previewKind={descriptor.presentation?.previews[key]} /></section>)}
              </>}
              {tab === "logs" && <pre className="whitespace-pre-wrap break-all text-[11px]">{logs.join("\n") || "暂无日志"}</pre>}
              {tab === "history" && history.map((h, i) => <details key={i} className="rounded border border-border p-2 text-xs"><summary>{h.time} · {h.status}</summary>{h.outputs && Object.entries(h.outputs).map(([key, v]) => <div key={key} className="mt-2"><span>{key}</span><OutputValue value={v} /></div>)}</details>)}
            </div>
            {running && <ProgressBar value={progress} />}
            <div className="border-t border-border pt-2 text-xs text-muted-foreground">{status} · {elapsed} ms</div>
          </div>
        </div>
        <div className="flex justify-between border-t border-border px-5 py-3">
          <Button variant="ghost" size="sm" disabled={running} onClick={() => { setInputs({}); setParams(defaults()); setOutputs(undefined); setError(""); }}>
            <RotateCcw className="mr-1 h-3.5 w-3.5" />重置参数</Button>
          <div className="flex gap-2"><Button variant="outline" size="sm" onClick={close}>关闭</Button>
            {running ? <Button variant="outline" size="sm" onClick={() => controller.current?.abort()}><Square className="mr-1 h-3.5 w-3.5" />取消</Button> :
              <Button size="sm" disabled={busy} onClick={() => void run()}><Play className="mr-1 h-3.5 w-3.5" />{busy ? "已有任务运行中" : "运行"}</Button>}
          </div>
        </div>
      </div>
    </div>
  );
}
