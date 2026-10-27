import { create } from "zustand";
import type { AgentRequest, ReviewedPlan, SolveRequest } from "@/flow/agentPlan";
import type { SolveAttempt, SolveCandidate, SolveProgress } from "@/flow/agentSolve";
import type { AiRetryIssue } from "@/flow/agentRequest";

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
  solveProgress: SolveProgress | null;
  retryPrompt: AiRetryIssue | null;
  setRetryPrompt: (issue: AiRetryIssue | null) => void;
  retryAI: () => void;
  setSolveProgress: (progress: SolveProgress) => void;
  recordAttempt: (attempt: SolveAttempt) => void;
  setSolveResult: (status: AgentState["solveStatus"], candidates?: SolveCandidate[]) => void;
  pendingPlan: AgentRequest | null;
  focusNodeIds: string[];
  focusNodes: (ids: string[]) => void;
  launch: (plan: ReviewedPlan) => void;
  launchSolve: (request: SolveRequest) => boolean;
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
  solveStatus: "idle", candidates: [], attempts: [], solveProgress: null, retryPrompt: null,
  setRetryPrompt: (retryPrompt) => set({ retryPrompt }),
  retryAI: () => { if (get().running && !get().cancelling && get().retryPrompt) set({ retryPrompt: null }); },
  setSolveProgress: (solveProgress) => set({ solveProgress }),
  recordAttempt: (attempt) => set((s) => ({ attempts: [...s.attempts, attempt] })),
  setSolveResult: (solveStatus, candidates) => set((s) => ({ solveStatus, candidates: candidates ?? s.candidates })),
  focusNodes: (focusNodeIds) => set({ focusNodeIds }),
  launch: (plan) => {
    if (!get().running && !get().pendingPlan) set({ pendingPlan: structuredClone(plan) });
  },
  launchSolve: (request) => {
    if (get().running || get().pendingPlan) return false;
    set({ pendingPlan: structuredClone(request) });
    return true;
  },
  clearPending: () => set({ pendingPlan: null }),
  start: () => set({ running: true, cancelling: false, cancelled: false, focusNodeIds: [], steps: [], notes: "", error: "", solveStatus: "idle", candidates: [], attempts: [], solveProgress: null, retryPrompt: null }),
  requestCancel: () => set({ cancelling: true, pendingPlan: null }),
  pushStep: (step) => set((state) => ({ steps: [...state.steps, step] })),
  finish: (notes, cancelled = false) => set((s) => ({ running: false, cancelling: false, cancelled, notes, retryPrompt: null,
    solveProgress: s.solveProgress ? { ...s.solveProgress, stage: "finished", message: notes } : null })),
  setError: (error) => set({ running: false, cancelling: false, error, retryPrompt: null }),
  reset: () => { if (!get().running) set({ steps: [], notes: "", error: "", pendingPlan: null, cancelling: false, cancelled: false, focusNodeIds: [], solveStatus: "idle", candidates: [], attempts: [], solveProgress: null, retryPrompt: null }); },
}));
