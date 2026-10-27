import { useGraphStore } from "@/store/graph";
import type { NodeDescriptor } from "./types";
import catalog from "./nodeCatalog.generated.json";

/** Execution is available only in the Tauri desktop runtime. */
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
/** Generated from the Rust registry with `pnpm generate:catalog`. */
export const mockDescriptors = catalog as unknown as NodeDescriptor[];
const DEMO_IMAGE =
  "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='120'%3E%3Crect width='100%25' height='100%25' fill='%23000'/%3E%3Crect x='16' y='16' width='88' height='88' fill='%23fff'/%3E%3Ctext x='60' y='66' font-size='18' text-anchor='middle'%3EQR%3C/text%3E%3C/svg%3E";

/** Seed a demo graph (browser preview only) so the canvas isn't empty. */
export function seedDemo() {
  const g = useGraphStore.getState();
  if (g.nodes.length > 0) return;
  const byId = (id: string) => mockDescriptors.find((d) => d.id === id)!;
  const a = g.addNode(byId("text_input"), { x: 40, y: 80 });
  const b = g.addNode(byId("base64_decode"), { x: 300, y: 80 });
  const c = g.addNode(byId("text_output"), { x: 560, y: 80 });
  g.onConnect({ source: a, sourceHandle: "text", target: b, targetHandle: "text" });
  g.onConnect({ source: b, sourceHandle: "text", target: c, targetHandle: "text" });
  g.setSelected(b);
  g.updateRuntime(b, {
    status: "done",
    outputs: { text: { type: "text", value: "flag{misc_flow_is_fun}" } },
  });
  g.updateRuntime(c, {
    status: "done",
    outputs: { value: { type: "text", value: "flag{misc_flow_is_fun}" } },
  });
  const qr = g.addNode(byId("qr_encode"), { x: 300, y: 260 });
  g.updateRuntime(qr, {
    status: "done",
    outputs: { image: { type: "image", value: DEMO_IMAGE } },
  });
}
