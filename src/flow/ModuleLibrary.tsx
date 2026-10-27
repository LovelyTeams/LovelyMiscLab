import { useMemo, useState } from "react";
import { ChevronDown, ChevronRight, RotateCw } from "lucide-react";

import type { NodeDescriptor, PortSpec } from "@/lib/types";
import { useDescriptorStore } from "@/store/descriptors";
import { usePaletteDrag } from "@/store/paletteDrag";

import { categoryLabel, searchNodes } from "@/lib/catalog";
import { nodeIcon } from "./nodeIcons";
import { portColor } from "./portColors";

const COLLAPSE_KEY = "misclab-collapsed-categories";

function describe(d: NodeDescriptor) {
  return d.description || "";
}

function Dots({ ports }: { ports: PortSpec[] }) {
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

export function ModuleLibrary() {
  const list = useDescriptorStore((s) => s.list);
  const categories = useDescriptorStore((s) => s.categories);
  const [limits, setLimits] = useState<Record<string, number>>({});
  const [q, setQ] = useState("");
  const [collapsed, setCollapsed] = useState<Set<string>>(() => {
    try {
      const saved = localStorage.getItem(COLLAPSE_KEY + "-v2");
      return new Set(saved ? JSON.parse(saved) : categories.filter((c) => !c.parentId && c.id !== "io").map((c) => c.id));
    } catch {
      return new Set();
    }
  });

  const toggle = (c: string) =>
    setCollapsed((prev) => {
      const next = new Set(prev);
      next.has(c) ? next.delete(c) : next.add(c);
      try {
        localStorage.setItem(COLLAPSE_KEY + "-v2", JSON.stringify([...next]));
      } catch {
        /* ignore */
      }
      return next;
    });

  const grouped = useMemo(() => {
    const filtered = searchNodes(list, q);
    const map = new Map<string, NodeDescriptor[]>();
    for (const d of filtered) {
      const id = d.categoryId || d.category;
      const arr = map.get(id) ?? [];
      arr.push(d);
      map.set(id, arr);
    }
    return Array.from(map.entries()).sort((a, b) => (categories.find((c) => c.id === a[0])?.order ?? 999) - (categories.find((c) => c.id === b[0])?.order ?? 999));
  }, [list, q, categories]);

  const onPointerDown = (e: React.PointerEvent, d: NodeDescriptor) => {
    if (e.button !== 0) return;
    e.preventDefault();
    usePaletteDrag.getState().start(d, e.clientX, e.clientY);
  };

  const searching = q.trim().length > 0;

  return (
    <div className="flex h-full flex-col bg-card">
      <div className="flex items-center justify-between border-b border-border px-3 py-2">
        <span className="text-xs font-semibold">模块库</span>
        <RotateCw className="h-3.5 w-3.5 text-muted-foreground" />
      </div>
      <div className="border-b border-border p-2">
        <input
          value={q}
          onChange={(e) => { setQ(e.target.value); setLimits({}); }}
          placeholder="搜索名称 / ID / 别名…"
          className="w-full rounded-md border border-input bg-background px-2 py-1.5 text-xs focus:outline-none focus:ring-1 focus:ring-ring"
        />
      </div>
      <div className="thin-scrollbar flex-1 overflow-y-auto p-2">
        {grouped.map(([category, nodes]) => {
          const isCollapsed = !searching && collapsed.has(category);
          return (
            <div key={category} className="mb-2">
              <button
                onClick={() => toggle(category)}
                className="flex w-full items-center gap-1 rounded px-1 py-1 text-[11px] font-semibold text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
              >
                {isCollapsed ? (
                  <ChevronRight className="h-3 w-3 shrink-0" />
                ) : (
                  <ChevronDown className="h-3 w-3 shrink-0" />
                )}
                <span className="flex-1 truncate text-left">{categories.find((c) => c.id === category)?.name ?? category}</span>
                <span className="text-[10px] opacity-60">{nodes.length}</span>
              </button>
              {!isCollapsed && (
                <div className="space-y-1 p-1">
                  {nodes.slice(0, limits[category] ?? 12).map((d) => {
                    const Icon = nodeIcon(d.id, d.category);
                    const desc = describe(d);
                    return (
                      <div
                        key={d.id}
                        onPointerDown={(e) => onPointerDown(e, d)}
                        title={desc ? `${d.displayName}\n${desc}` : d.displayName}
                        className="cursor-grab touch-none select-none rounded-lg border border-border bg-background p-2 transition-all hover:border-primary hover:shadow-sm active:cursor-grabbing"
                      >
                        <div className="flex items-center gap-1.5">
                          <span
                            className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md"
                            style={{ background: `${d.color}18`, color: d.color }}
                          >
                            <Icon className="h-3.5 w-3.5" />
                          </span>
                          <span className="min-w-0 flex-1 truncate text-[11px] font-medium">
                            {d.displayName}
                          </span>
                          <span className="flex shrink-0 items-center gap-1 text-[9px] text-muted-foreground">
                            <Dots ports={d.inputs} />
                            <span className="opacity-50">→</span>
                            <Dots ports={d.outputs} />
                          </span>
                        </div>
                        {desc && (
                          <div className="mt-1 line-clamp-2 pl-[30px] text-[10px] leading-snug text-muted-foreground">
                            {desc}
                          </div>
                        )}
                        <div className="mt-1 truncate pl-[30px] text-[9px] text-muted-foreground">{categoryLabel(d, categories)}</div>
                      </div>
                    );
                  })}
                  {nodes.length > (limits[category] ?? 12) && <button className="w-full py-2 text-[11px] text-primary" onClick={() => setLimits((p) => ({ ...p, [category]: (p[category] ?? 12) + 24 }))}>继续显示（剩余 {nodes.length - (limits[category] ?? 12)}）</button>}
                </div>
              )}
            </div>
          );
        })}
        {grouped.length === 0 && (
          <div className="p-2 text-xs text-muted-foreground">无匹配模块</div>
        )}
      </div>
      <div className="border-t border-border px-3 py-1.5 text-[10px] text-muted-foreground">
        拖拽或点击添加到画布
      </div>
    </div>
  );
}
