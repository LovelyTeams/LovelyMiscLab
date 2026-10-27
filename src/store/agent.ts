import { create } from "zustand";
import type { ReviewedPlan } from "@/flow/agentPlan";
import type { SolveAttempt, SolveCandidate } from "@/flow/agentSolve";

export interface AgentStep {
  kind: "add" | "connect" | "run" | "result" | "error" | "done" | "plan" | "evidence" | "diagnose" | "search" | "change" | "verify";
  text: string;
  detail?: string;
  ok?: boolean;
}

interface AgentState {
  running: boolean;
  cancelling: boolean;
  cancelled: boolean;
  steps: AgentStep[];
  notes: string;
  error: string;
  solveStatus: "idle" | "working" | "candidate" | "verified" | "unresolved" | "cancelled";
  candidates: SolveCandidate[];
  attempts: SolveAttempt[];
  recordAttempt: (attempt: SolveAttempt) => void;
  setSolveResult: (status: AgentState["solveStatus"], candidates?: SolveCandidate[]) => void;
  pendingPlan: ReviewedPlan | null;
  focusNodeIds: string[];
  focusNodes: (ids: string[]) => void;
  launch: (plan: ReviewedPlan) => void;
  clearPending: () => void;
  start: () => void;
  requestCancel: () => void;
  pushStep: (step: AgentStep) => void;
  finish: (notes: string, cancelled?: boolean) => void;
  setError: (error: string) => void;
  reset: () => void;
}

export const useAgentStore = create<AgentState>((set, get) => ({
  running: false, cancelling: false, cancelled: false, steps: [], notes: "", error: "", pendingPlan: null, focusNodeIds: [],
  solveStatus: "idle", candidates: [], attempts: [],
  recordAttempt: (attempt) => set((s) => ({ attempts: [...s.attempts, attempt] })),
  setSolveResult: (solveStatus, candidates) => set((s) => ({ solveStatus, candidates: candidates ?? s.candidates })),
  focusNodes: (focusNodeIds) => set({ focusNodeIds }),
  launch: (plan) => {
    if (!get().running && !get().pendingPlan) set({ pendingPlan: structuredClone(plan) });
  },
  clearPending: () => set({ pendingPlan: null }),
  start: () => set({ running: true, cancelling: false, cancelled: false, focusNodeIds: [], steps: [], notes: "", error: "", solveStatus: "idle", candidates: [], attempts: [] }),
  requestCancel: () => set({ cancelling: true, pendingPlan: null }),
  pushStep: (step) => set((state) => ({ steps: [...state.steps, step] })),
  finish: (notes, cancelled = false) => set({ running: false, cancelling: false, cancelled, notes }),
  setError: (error) => set({ running: false, cancelling: false, error }),
  reset: () => { if (!get().running) set({ steps: [], notes: "", error: "", pendingPlan: null, cancelling: false, cancelled: false, focusNodeIds: [], solveStatus: "idle", candidates: [], attempts: [] }); },
}));
