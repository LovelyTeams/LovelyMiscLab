import { api } from "@/lib/bindings";
import type { ProgressMsg } from "@/lib/types";

export interface AiRetryIssue { message: string; attempts: number; retryAfterMs?: number }
export interface AiRequestOptions {
  onRetryExhausted?: (issue: AiRetryIssue) => Promise<void>;
  retryDelaysMs?: readonly number[];
}
const defaultDelays = [1000, 3000];
const cancelled = () => new Error("已取消 AI 请求");

/** Prefer explicit transport metadata; retain compatibility with older desktop
 * errors without retrying invalid workflow plans or authentication failures. */
export function aiRetryIssue(error: unknown): Omit<AiRetryIssue, "attempts"> | null {
  if (!error || typeof error !== "object") return null;
  const e = error as { code?: string; message?: string; retryable?: boolean; retryAfterMs?: number };
  const message = e.message ?? "AI 网络请求失败";
  const legacyBodyError = e.code === "ai_parse" && /AI 响应解析失败.*(?:error decoding response body|unexpected end|EOF)/i.test(message);
  if (!["ai_http", "ai_network", "ai_timeout", "ai_response"].includes(e.code ?? "") && !(e.code === "ai_parse" && (e.retryable === true || legacyBodyError))) return null;
  const status = message.match(/\b([45]\d{2})\b/);
  if (e.retryable === false || (status && ![408, 429].includes(Number(status[1])) && Number(status[1]) < 500)) return null;
  if (e.retryable !== true && !status && !legacyBodyError && !/超时|连接|网络|timeout|timed out|connect|network|dns|reset|sending request|body/i.test(message)) return null;
  return { message, ...(Number.isFinite(e.retryAfterMs) && e.retryAfterMs! > 0 ? { retryAfterMs: e.retryAfterMs } : {}) };
}

/** Cancellable backoff, including long provider Retry-After values. */
async function retryDelay(ms: number, signal: AbortSignal): Promise<void> {
  let remaining = Math.max(0, ms);
  do {
    if (signal.aborted) throw cancelled();
    const delay = Math.min(remaining, 60000);
    await new Promise<void>((resolve, reject) => {
      const onAbort = () => { clearTimeout(timer); signal.removeEventListener("abort", onAbort); reject(cancelled()); };
      const timer = setTimeout(() => { signal.removeEventListener("abort", onAbort); resolve(); }, delay);
      signal.addEventListener("abort", onAbort, { once: true });
    });
    remaining -= delay;
  } while (remaining > 0);
}

/** Every attempt owns its job ID; late events cannot affect the next request. */
async function requestOnce<T>(
  operation: (emit: (event: ProgressMsg) => void) => Promise<T>,
  signal: AbortSignal,
  onProgress: (message: string) => void,
): Promise<T> {
  if (signal.aborted) throw cancelled();
  let job: string | undefined;
  let cancelledJob: string | undefined;
  let active = true;
  let rejectCancelled: (reason: Error) => void = () => {};
  const cancellation = new Promise<never>((_, reject) => { rejectCancelled = reject; });
  const cancelJob = () => {
    if (job && cancelledJob !== job) {
      cancelledJob = job;
      void api.cancelJob(job).catch(() => {});
    }
  };
  const cancel = () => { cancelJob(); rejectCancelled(cancelled()); };
  signal.addEventListener("abort", cancel, { once: true });
  try {
    const result = await Promise.race([operation((event) => {
      if (event.kind === "jobStarted") { job = event.job; if (signal.aborted || !active) cancelJob(); }
      if (active && !signal.aborted && event.kind === "log") onProgress(event.message);
    }), cancellation]);
    if (signal.aborted) throw cancelled();
    return result;
  } finally { active = false; signal.removeEventListener("abort", cancel); }
}

/** Only model requests are retried; this wrapper never executes canvas nodes. */
export async function agentRequest<T>(operation: (emit: (event: ProgressMsg) => void) => Promise<T>,
  signal: AbortSignal, onProgress: (message: string) => void, options: AiRequestOptions = {}): Promise<T> {
  const delays = options.retryDelaysMs ?? defaultDelays;
  let retries = 0;
  for (;;) {
    if (signal.aborted) throw cancelled();
    try { return await requestOnce(operation, signal, onProgress); }
    catch (error) {
      if (signal.aborted) throw cancelled();
      const issue = aiRetryIssue(error);
      if (!issue) throw error;
      if (retries < delays.length) {
        const delay = Math.max(delays[retries], issue.retryAfterMs ?? 0);
        onProgress(`AI 请求暂时失败：${issue.message} ${Math.ceil(delay / 1000)} 秒后自动重试（${++retries}/${delays.length}）`);
        await retryDelay(delay, signal);
      } else {
        if (!options.onRetryExhausted) throw error;
        onProgress("自动重试仍未成功，可在画布点击“重试 AI 请求”继续；当前图和证据已保留。");
        await options.onRetryExhausted({ ...issue, attempts: retries + 1 });
        if (signal.aborted) throw cancelled();
        onProgress("正在重试当前 AI 请求，保留已有图和执行证据…");
        if (issue.retryAfterMs) await retryDelay(issue.retryAfterMs, signal);
        retries = 0;
      }
    }
  }
}
