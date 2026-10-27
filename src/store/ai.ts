import { create } from "zustand";

/** Whether the "AI 生成流程" dialog is open. */
export const useAiStore = create<{ open: boolean; mode: "generate" | "explain" | "repair" | "solve"; setOpen: (o: boolean) => void; setMode: (mode: "generate" | "explain" | "repair" | "solve") => void }>((set) => ({
  open: false,
  mode: "generate",
  setOpen: (open) => set({ open }),
  setMode: (mode) => set({ mode }),
}));
