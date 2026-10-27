import { useRef, useState } from "react";
import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import type { PortSpec, PortValue } from "@/lib/types";
import { defaultFormat, type InputDraft } from "@/lib/typedInputs";

const cls = "nodrag nowheel nopan w-full rounded border border-input bg-background px-2 py-1.5 text-xs";
const formatsLabel: Record<string, string> = { text: "文本", hex: "Hex 十六进制", base64: "Base64", utf8: "UTF-8 字节", file: "文件字节", number: "数值", bool: "布尔", json: "JSON", stringList: "字符串列表", candidates: "候选列表", image: "图片", artifact: "文件产物", fingerprint: "指纹对象" };
export function PortInputEditor({ port, draft, onChange, disabled = false }: { port: PortSpec; draft?: InputDraft; onChange: (value: InputDraft) => void; disabled?: boolean }) {
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const generation = useRef(0);
  const d = draft ?? { text: "", format: defaultFormat(port.type) };
  const format = d.format ?? defaultFormat(port.type);
  const update = (patch: Partial<InputDraft>) => { generation.current++; setError(""); onChange({ ...d, value: undefined, ...patch }); };
  const formats = port.type === "bytes" ? ["hex", "base64", "utf8", "file"] : port.type === "any" ? ["text", "hex", "base64", "utf8", "number", "bool", "json", "stringList", "candidates", "image", "artifact", "file"] : [port.type];
  const file = async (f?: File) => {
    if (!f) return;
    const seq = ++generation.current;
    setError(""); setLoading(true);
    try {
      if (f.size > 16 * 1024 * 1024) throw new Error("直接输入文件上限为 16 MiB；大型文件请使用画布文件导入节点");
      let value: PortValue;
      if (format === "image") {
        const url = await new Promise<string>((resolve, reject) => { const r = new FileReader(); r.onload = () => resolve(String(r.result)); r.onerror = reject; r.readAsDataURL(f); });
        if (!url.startsWith("data:image/")) throw new Error("请选择图片文件");
        value = { type: "image", value: url };
      } else {
        const bytes = Array.from(new Uint8Array(await f.arrayBuffer()));
        if (format === "artifact") {
          if (!inTauri) throw new Error("导入产物需要桌面应用");
          const a = await api.artifactPut(f.name, f.type || "application/octet-stream", bytes);
          value = { type: "artifact", value: a.id };
        } else value = { type: "bytes", value: bytes };
      }
      if (seq === generation.current) onChange({ ...d, enabled: true, value, text: f.name });
    } catch (e) { if (seq === generation.current) setError(e instanceof Error ? e.message : String(e)); }
    finally { if (seq === generation.current) setLoading(false); }
  };
  return <fieldset disabled={disabled || loading} className="min-w-0 space-y-1.5">
    <label className="flex items-center gap-2 text-xs font-medium">
      {!port.required && <input type="checkbox" checked={d.enabled === true} onChange={(e) => update({ enabled: e.target.checked })} />}
      {port.label} <span className="font-normal text-muted-foreground">{port.type} · {port.required ? "必填" : "可选"}</span>
    </label>
    {(port.required || d.enabled) && <>
      {formats.length > 1 && <select aria-label={`${port.label}格式`} value={format} onChange={(e) => update({ format: e.target.value, text: "" })} className={cls}>{formats.map((f) => <option key={f} value={f}>{formatsLabel[f] ?? f}</option>)}</select>}
      {format === "bool" ? <select aria-label={port.label} className={cls} value={d.text || "false"} onChange={(e) => update({ text: e.target.value })}><option>false</option><option>true</option></select> : format === "number" ? <input aria-label={port.label} className={cls} type="number" value={d.text} onChange={(e) => update({ text: e.target.value })} /> : <textarea aria-label={port.label} className={cls} rows={3} value={d.value ? "" : d.text} placeholder={d.value ? `已选择 ${d.text}` : format === "stringList" ? "每行一项或 JSON 字符串数组" : format === "candidates" ? '[{"text":"候选","score":1}]' : `${format} 输入`} onChange={(e) => update({ text: e.target.value })} />}
      {["file", "image", "artifact"].includes(format) && <input aria-label={`${port.label}文件`} type="file" accept={format === "image" ? "image/*" : undefined} className="block w-full text-xs" onChange={(e) => void file(e.target.files?.[0])} />}
    </>}
    {port.description && <p className="text-[10px] text-muted-foreground">{port.description}</p>}
    {error && <p className="text-xs text-destructive">{error}</p>}
    {loading && <p className="text-xs text-muted-foreground">正在读取文件…</p>}
  </fieldset>;
}
