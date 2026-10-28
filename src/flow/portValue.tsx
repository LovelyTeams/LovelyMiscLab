import { useEffect, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import type { ArtifactMetadata, PortValue } from "@/lib/types";
import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import { writeClipboardText } from "@/lib/clipboard";
import { useImageViewer } from "@/store/imageViewer";

export function valueText(v: PortValue): string {
  switch (v.type) {
    case "none": return "";
    case "bytes": return bytesToHex(v.value, v.value.length);
    case "text": case "image": case "artifact": return v.value;
    case "number": case "bool": return String(v.value);
    case "stringList": return v.value.join("\n");
    case "candidates": return v.value.map((c) => c.score.toFixed(2) + "  " + c.text + (c.note ? "  (" + c.note + ")" : "")).join("\n");
    default: return JSON.stringify(v.value, null, 2);
  }
}
export function bytesToHex(bytes: number[], limit = 512): string {
  const shown = bytes.slice(0, limit).map((b) => b.toString(16).padStart(2, "0")).join(" ");
  return bytes.length > limit ? shown + " …" : shown;
}
const preClass = "max-h-56 select-text overflow-auto whitespace-pre-wrap break-all rounded bg-background p-2 font-mono text-[10px] leading-snug";
const buttonClass = "rounded border border-border px-2 py-0.5 text-[10px] hover:bg-accent";

function JsonTree({ value, depth = 0 }: { value: unknown; depth?: number }) {
  const [open, setOpen] = useState(depth === 0);
  const [count, setCount] = useState(50);
  if (!value || typeof value !== "object") return <span className="break-all">{String(JSON.stringify(value)).slice(0, 4000)}</span>;
  const entries = Object.entries(value);
  if (depth > 12) return <span>深层结构请复制或导出查看</span>;
  return <div className="pl-2">
    <button className="text-primary" onClick={() => setOpen(!open)}>{open ? "▾" : "▸"} {Array.isArray(value) ? "数组" : "对象"} ({entries.length})</button>
    {open && <div className="border-l border-border pl-2">{entries.slice(0, count).map(([k, v]) => <div key={k} className="py-0.5"><span className="font-semibold">{k}: </span><JsonTree value={v} depth={depth + 1} /></div>)}
      {entries.length > count && <button onClick={() => setCount(count + 50)} className="text-primary">继续显示</button>}</div>}
  </div>;
}

function ArtifactValue({ id, compact }: { id: string; compact: boolean }) {
  const [meta, setMeta] = useState<ArtifactMetadata>();
  const [error, setError] = useState("");
  const [url, setUrl] = useState("");
  const [loading, setLoading] = useState(false);
  const revision = useRef(0);
  useEffect(() => {
    const seq = ++revision.current;
    setMeta(undefined); setError(""); setUrl(""); setLoading(false);
    if (inTauri) void api.artifactMetadata(id).then((m) => { if (seq === revision.current) setMeta(m); }).catch((e) => { if (seq === revision.current) setError(String(e.message ?? e)); });
    return () => { revision.current++; };
  }, [id]);
  useEffect(() => () => { if (url) URL.revokeObjectURL(url); }, [url]);
  const preview = async () => {
    if (!meta || loading) return;
    const seq = revision.current;
    setLoading(true); setError("");
    try {
      if (meta.size > 16 * 1024 * 1024) throw new Error("预览上限 16 MiB，请导出查看完整产物");
      const chunks: Uint8Array<ArrayBuffer>[] = [];
      for (let offset = 0; offset < meta.size; offset += 1024 * 1024) {
        if (seq !== revision.current) return;
        chunks.push(new Uint8Array(await api.artifactRead(id, offset, Math.min(1024 * 1024, meta.size - offset))));
      }
      if (seq === revision.current) setUrl(URL.createObjectURL(new Blob(chunks, { type: meta.mime })));
    } catch (e) { if (seq === revision.current) setError(e instanceof Error ? e.message : String(e)); }
    finally { if (seq === revision.current) setLoading(false); }
  };
  const exportFile = async () => {
    try { const path = await save({ defaultPath: meta?.name ?? "output.bin" }); if (path) await api.artifactExport(id, path); }
    catch (e) { setError(String((e as { message?: string }).message ?? e)); }
  };
  return <div className="space-y-1 text-[10px]">
    <div className="truncate font-medium">{meta?.name ?? "文件产物"}{meta && " · " + meta.size + " 字节"}</div>
    {!compact && <><code className="break-all">{id}</code>{meta && <p className="break-all text-muted-foreground">SHA-256: {meta.sha256}</p>}
      <div className="flex gap-2"><button disabled={!inTauri} className={buttonClass} onClick={() => void exportFile()}>导出文件</button>
        {meta && /^(image|audio|video)\//.test(meta.mime) && <button disabled={loading} className={buttonClass} onClick={() => void preview()}>{loading ? "读取中…" : "加载预览"}</button>}</div>
      {url && (meta?.mime.startsWith("audio/") ? <audio className="w-full" controls src={url} /> : meta?.mime.startsWith("video/") ? <video className="max-h-56 w-full" controls src={url} /> : <img className="max-h-56 w-full object-contain" src={url} alt={meta?.name} />)}</>}
    {error && <p className="text-destructive">{error}</p>}
  </div>;
}

/** Typed, bounded rendering shared by canvas, inspector, standalone and history. */
export function OutputValue({ value, compact = false, previewKind }: { value: PortValue; compact?: boolean; previewKind?: string }) {
  const [limit, setLimit] = useState(20000);
  const [notice, setNotice] = useState("");
  const [tree, setTree] = useState(previewKind !== "table");
  useEffect(() => { setLimit(20000); setNotice(""); }, [value]);
  const copy = async () => {
    try { await writeClipboardText(valueText(value)); setNotice("已复制"); }
    catch (e) { setNotice(e instanceof Error ? e.message : "复制失败"); }
  };
  const download = () => {
    const bytes = value.type === "bytes" ? new Uint8Array(value.value) : valueText(value);
    const url = URL.createObjectURL(new Blob([bytes], { type: value.type === "bytes" ? "application/octet-stream" : "text/plain;charset=utf-8" }));
    const a = document.createElement("a"); a.href = url; a.download = value.type === "bytes" ? "output.bin" : "output.txt"; a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  };
  let body: React.ReactNode;
  if (value.type === "none") body = <span className="text-[10px] text-muted-foreground">（无输出）</span>;
  else if (value.type === "artifact") body = <ArtifactValue id={value.value} compact={compact} />;
  else if (value.type === "image") body = <img src={value.value} alt="输出预览" title="点击查看大图" onClick={() => useImageViewer.getState().show(value.value)} className={(compact ? "max-h-28" : "max-h-56") + " max-w-full cursor-zoom-in rounded border border-border object-contain"} />;
  else if (value.type === "bytes") body = <div><p className="text-[10px] text-muted-foreground">{value.value.length} 字节 · Hex</p><pre className={preClass}>{bytesToHex(value.value, compact ? 32 : Math.floor(limit / 3)) || "（空）"}</pre>{!compact && value.value.length * 3 > limit && <button className={buttonClass} onClick={() => setLimit(limit + 20000)}>查看更多字节</button>}</div>;
  else if (value.type === "json" || value.type === "fingerprint") {
    const array = Array.isArray(value.value) ? value.value : null;
    const table = array && array.length > 0 && array.every((v) => v && typeof v === "object" && !Array.isArray(v));
    const keys = table ? [...new Set(array!.slice(0, 50).flatMap(Object.keys))].slice(0, 12) : [];
    body = compact ? <pre className={preClass}>{JSON.stringify(value.value).slice(0, 240)}</pre> : <div className="max-h-64 overflow-auto text-[10px]">
      {table && <button className={buttonClass} onClick={() => setTree(!tree)}>{tree ? "表格" : "JSON 树"}</button>}
      {table && !tree ? <><table className="w-full text-left"><thead><tr>{keys.map((k) => <th key={k} className="border-b p-1">{k}</th>)}</tr></thead><tbody>{array!.slice(0, 50).map((row, i) => <tr key={i}>{keys.map((k) => <td key={k} className="max-w-48 truncate border-b p-1">{String(typeof row[k] === "object" ? JSON.stringify(row[k]) : row[k] ?? "").slice(0, 200)}</td>)}</tr>)}</tbody></table><p>预览前 50 行、12 列；完整内容可复制或导出。</p></> : <JsonTree value={value.value} />}
    </div>;
  } else {
    const text = valueText(value);
    body = <><pre className={preClass}>{text.slice(0, compact ? 240 : limit) || "（空）"}</pre>{!compact && text.length > limit && <button className={buttonClass} onClick={() => setLimit(limit + 20000)}>继续显示（共 {text.length} 字符）</button>}</>;
  }
  return <div className={"nodrag nowheel nopan min-w-0 space-y-1 " + (compact ? "max-h-36 overflow-hidden" : "")}>
    <div data-text-region className="select-text">{body}</div>
    {!compact && !["artifact", "image", "none"].includes(value.type) && <div className="flex items-center gap-2"><button className={buttonClass} onClick={() => void copy()}>复制</button><button className={buttonClass} onClick={download}>导出</button><span className="text-[10px] text-muted-foreground">{notice}</span></div>}
  </div>;
}
