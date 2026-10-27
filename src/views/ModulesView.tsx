import { useMemo, useState } from "react";
import { Play, Plus, Trash2 } from "lucide-react";

import { api } from "@/lib/bindings";
import { inTauri } from "@/lib/devMocks";
import { cn } from "@/lib/utils";
import { categoryLabel, matchesCategory, searchNodes } from "@/lib/catalog";
import type { NodeDescriptor, PortSpec } from "@/lib/types";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";
import { useViewStore } from "@/store/view";
import { nodeIcon } from "@/flow/nodeIcons";
import { portColor } from "@/flow/portColors";

import { ModuleRunDialog } from "./ModuleRunDialog";

function Ports({ ports }: { ports: PortSpec[] }) {
  if (ports.length === 0) return <span className="text-muted-foreground/40">无</span>;
  return (
    <span className="flex gap-0.5">
      {ports.map((p) => (
        <span
          key={p.name}
          title={`${p.label}: ${p.type}`}
          className="h-1.5 w-1.5 rounded-full"
          style={{ background: portColor(p.type) }}
        />
      ))}
    </span>
  );
}

export function ModulesView() {
  const list = useDescriptorStore((s) => s.list);
  const setDescriptors = useDescriptorStore((s) => s.setDescriptors);
  const addNode = useGraphStore((s) => s.addNode);
  const setView = useViewStore((s) => s.setView);
  const [q, setQ] = useState("");
  const [cat, setCat] = useState("");
  const [subcat, setSubcat] = useState("");
  const [page, setPage] = useState(0);
  const [runModule, setRunModule] = useState<NodeDescriptor | null>(null);

  const catalog = useDescriptorStore((s) => s.categories);
  const categories = catalog.filter((c) => !c.parentId);

  const filtered = useMemo(() => {
    return searchNodes(list.filter((d) => matchesCategory(d, subcat || cat)), q);
  }, [list, q, cat, subcat]);
  const maxPage = Math.max(0, Math.ceil(filtered.length / 50) - 1);
  const currentPage = Math.min(page, maxPage);

  const addToCanvas = (d: NodeDescriptor) => {
    addNode(d, { x: 220 + Math.random() * 120, y: 140 + Math.random() * 120 });
    setView("canvas");
  };

  const removeModule = async (d: NodeDescriptor) => {
    if (inTauri) {
      // Script nodes (id `script_…`) and composite modules (`mod_…`) have separate stores.
      if (d.id.startsWith("script_")) await api.deleteScriptModule(d.id);
      else await api.deleteCompositeModule(d.id);
      setDescriptors(await api.listNodeDescriptors());
    } else {
      setDescriptors(useDescriptorStore.getState().list.filter((x) => x.id !== d.id));
    }
  };

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-border p-4">
        <h1 className="text-lg font-semibold">模块库</h1>
        <p className="text-xs text-muted-foreground">
          浏览全部模块 — 添加到画布编排流程，或直接单独调用执行。
        </p>
        <div className="mt-3 flex flex-wrap items-center gap-2">
          <input
            value={q}
            onChange={(e) => { setQ(e.target.value); setPage(0); }}
            placeholder="搜索名称 / ID / 别名…"
            className="w-64 rounded-md border border-input bg-background px-3 py-1.5 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
          />
          <div className="flex flex-wrap gap-1">
            <button onClick={() => { setCat(""); setSubcat(""); setPage(0); }} className={cn("rounded-full px-3 py-1 text-xs", !cat ? "bg-primary text-primary-foreground" : "bg-secondary")}>全部</button>
            {categories.map((c) => (
              <button
                key={c.id}
                onClick={() => { setCat(c.id); setSubcat(""); setPage(0); }}
                className={cn(
                  "rounded-full px-3 py-1 text-xs transition-colors",
                  cat === c.id
                    ? "bg-primary text-primary-foreground"
                    : "bg-secondary text-muted-foreground hover:bg-accent hover:text-foreground"
                )}
              >
                {c.name}
              </button>
            ))}
          </div>
        </div>
      </div>

      {cat && <select aria-label="二级分类" value={subcat} onChange={(e) => { setSubcat(e.target.value); setPage(0); }} className="mx-4 mt-2 w-fit rounded border border-input bg-background px-2 py-1 text-xs"><option value="">全部分组</option>{catalog.filter((c) => c.parentId === cat).map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}</select>}
      <div className="flex-1 overflow-y-auto p-4">
        <div className="grid grid-cols-2 gap-3 lg:grid-cols-3 xl:grid-cols-4">
          {filtered.slice(currentPage * 50, (currentPage + 1) * 50).map((d) => {
            const Icon = nodeIcon(d.id, d.category);
            return (
              <div
                key={d.id}
                className="flex flex-col rounded-xl border border-border bg-card p-3 transition-all hover:border-primary hover:shadow-md"
              >
                <div className="flex items-start gap-2">
                  <span
                    className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg"
                    style={{ background: `${d.color}18`, color: d.color }}
                  >
                    <Icon className="h-5 w-5" />
                  </span>
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm font-medium">{d.displayName}</div>
                    <div className="text-[10px] text-muted-foreground">{categoryLabel(d, catalog)}</div>
                  </div>
                </div>
                <p className="mt-2 line-clamp-2 h-8 text-xs text-muted-foreground">
                  {d.description || "—"}
                </p>
                <div className="mt-2 flex items-center gap-1 text-[10px] text-muted-foreground">
                  <Ports ports={d.inputs} />
                  <span className="opacity-50">→</span>
                  <Ports ports={d.outputs} />
                </div>
                <div className="mt-3 flex gap-1">
                  <button
                    onClick={() => addToCanvas(d)}
                    className="flex flex-1 items-center justify-center gap-1 rounded-md bg-secondary py-1 text-[11px] hover:bg-accent"
                  >
                    <Plus className="h-3 w-3" /> 加入画布
                  </button>
                  <button
                    onClick={() => setRunModule(d)}
                    className="flex flex-1 items-center justify-center gap-1 rounded-md border border-border py-1 text-[11px] hover:bg-accent"
                  >
                    <Play className="h-3 w-3" /> 单独调用
                  </button>
                  {(d.id.startsWith("mod_") || d.id.startsWith("script_")) && (
                    <button
                      onClick={() => void removeModule(d)}
                      title="删除自定义模块"
                      className="flex items-center justify-center rounded-md border border-border px-2 py-1 text-[11px] text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                    >
                      <Trash2 className="h-3 w-3" />
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      <div className="flex items-center justify-between border-t border-border px-4 py-2 text-xs text-muted-foreground"><span>共 {filtered.length} 个模块 · 每页 50 项</span><div className="flex gap-3"><button disabled={currentPage === 0} onClick={() => setPage(currentPage - 1)}>上一页</button><span>{currentPage + 1} / {maxPage + 1}</span><button disabled={currentPage === maxPage} onClick={() => setPage(currentPage + 1)}>下一页</button></div></div>
      {runModule && (
        <ModuleRunDialog descriptor={runModule} onClose={() => setRunModule(null)} />
      )}
    </div>
  );
}
