import { api } from "@/lib/bindings";
import type { ProgressMsg } from "@/lib/types";

/** Cancel the backend even when its job ID arrives after the dialog closes. */
export async function agentRequest<T>(
  operation: (emit: (event: ProgressMsg) => void) => Promise<T>,
  signal: AbortSignal,
  onProgress: (message: string) => void,
): Promise<T> {
  let job: string | undefined;
  let cancelledJob: string | undefined;
  const cancel = () => {
    if (job && cancelledJob !== job) {
      cancelledJob = job;
      void api.cancelJob(job).catch(() => {});
    }
  };
  if (signal.aborted) throw new Error("已取消规划");
  signal.addEventListener("abort", cancel, { once: true });
  try {
    const result = await operation((event) => {
      if (event.kind === "jobStarted") { job = event.job; if (signal.aborted) cancel(); }
      if (!signal.aborted && event.kind === "log") onProgress(event.message);
    });
    if (signal.aborted) throw new Error("已取消规划");
    return result;
  } finally { signal.removeEventListener("abort", cancel); }
}
