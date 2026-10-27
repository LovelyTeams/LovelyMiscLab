import { useAgentStore } from "@/store/agent";
import { useDescriptorStore } from "@/store/descriptors";
import { useGraphStore } from "@/store/graph";
import { useRunStore } from "@/store/run";
import { preparePlan, type ReviewedPlan } from "./agentPlan";
import { checkAgentResults } from "./agentChecks";
import { executeNodeSet } from "./runner";
import { applyAgentPlan } from "./applyAgentPlan";
import { runSolver } from "./agentSolveRunner";

/** Ordinary plans execute their reviewed steps. Solve mode uses the separately
 * reviewed input scope and budget for a bounded feedback loop. */
export async function runAgent(reviewed: ReviewedPlan): Promise<void> {
  if (reviewed.solve) return runSolver(reviewed);
  const agent = useAgentStore.getState();
  if (agent.running) return;
  agent.start();
  try {
    const g = useGraphStore.getState();
    if (useRunStore.getState().running) throw new Error("请先停止当前运行，再执行计划");
    if (reviewed.revision !== g.runRevision) throw new Error("画布已变化，请重新预览并确认计划");
    const descriptors = useDescriptorStore.getState().byId;
    const plan = preparePlan(reviewed.graph, descriptors);
    useRunStore.getState().setMode("idle");
    const ids = applyAgentPlan(plan, reviewed.replace);
    agent.pushStep({ kind: "run", text: `执行已确认的 ${ids.size} 个节点` });
    const outcome = await executeNodeSet(new Set(ids.values()), "AI 计划执行");
    if (outcome.status === "cancelled") agent.finish(`${outcome.error || "已停止"}。已搭建节点保留，可撤销本次搭建。`, true);
    else if (outcome.status === "error") agent.setError(outcome.error || "执行失败，请查看节点错误和运行记录");
    else {
      const nodes = useGraphStore.getState().nodes;
      const results = Object.fromEntries([...ids].map(([key, id]) => {
        const node = nodes.find((n) => n.id === id)!;
        return [key, { status: node.data.status, outputs: node.data.outputs, error: node.data.error }];
      }));
      const checks = checkAgentResults(plan, results);
      for (const step of checks) {
        agent.pushStep(step);
        if (outcome.historyId) useRunStore.getState().appendHistoryEvent(outcome.historyId, {
          time: new Date().toLocaleTimeString(), level: step.ok ? "success" : "error", message: `自动测试：${step.text}`,
        });
      }
      const failed = checks.filter((step) => !step.ok);
      if (failed.length) {
        const error = `自动测试 ${failed.length} 项未通过：${failed.map((s) => s.text).join("；")}`;
        useRunStore.getState().setLastError(error);
        if (outcome.historyId) useRunStore.getState().finishHistory(outcome.historyId, { status: "error", error });
        agent.setError(error);
      } else {
        agent.pushStep({ kind: "done", text: `执行及自动测试通过（${checks.length} 项）` });
        agent.finish(plan.checks?.length ? "结果符合已确认的测试条件。" : "节点执行正常。未设置结果断言，请检查输出是否符合任务目标。");
      }
    }
  } catch (error) {
    agent.setError(error instanceof Error ? error.message : String(error));
  }
}
