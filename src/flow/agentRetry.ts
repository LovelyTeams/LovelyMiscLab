import { useAgentStore as A } from "@/store/agent";
import type { AiRetryIssue } from "./agentRequest";

/** Preserve the current call stack and graph ownership while awaiting a retry.
 * Cancellation, canvas edits and the solve deadline continue to apply. */
export function waitForAgentRetry(issue: AiRetryIssue, signal: AbortSignal): Promise<void> {
  if (signal.aborted) return Promise.reject(new Error("已取消 AI 请求"));
  return new Promise((resolve, reject) => {
    let off = () => {};
    const cleanup = () => {
      off(); signal.removeEventListener("abort", onAbort);
      if (A.getState().retryPrompt === issue) A.getState().setRetryPrompt(null);
    };
    const onAbort = () => { cleanup(); reject(new Error("已取消 AI 请求")); };
    signal.addEventListener("abort", onAbort, { once: true });
    // Subscribe before exposing the prompt so even an immediate retry is seen.
    off = A.subscribe((s) => {
      if (!s.retryPrompt) { cleanup(); signal.aborted ? reject(new Error("已取消 AI 请求")) : resolve(); }
    });
    A.getState().setRetryPrompt(issue);
  });
}
