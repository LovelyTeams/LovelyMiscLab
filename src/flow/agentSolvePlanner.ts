import { api, type GeneratedGraph } from "@/lib/bindings";
import type { NodeDescriptor } from "@/lib/types";
import { searchNodes } from "@/lib/catalog";
import { agentRequest } from "./agentRequest";
import { isSolverModule, prepareSolvePlan, solvePrompt, type SolveAttempt, type SolveOptions, type SolvePlanning } from "./agentSolve";

/** Decide first, query the same catalog as the module library, then provide only
 * real matching signatures plus the current graph's modules to the planner. */
export async function planSolveRound(options: SolveOptions, attempts: SolveAttempt[], previous: GeneratedGraph | undefined,
  descriptors: Record<string, NodeDescriptor>, signal: AbortSignal, report: (message: string) => void,
  onPlanning: (planning: SolvePlanning) => void = () => {}) {
  const prompt = solvePrompt(options, attempts);
  let planning: SolvePlanning | undefined, feedback = "";
  for (let searchAttempt = 0; searchAttempt < 2; searchAttempt++) {
    report("分析实际证据，判断需要修复或继续处理的节点…");
    const decision = await agentRequest((emit) => api.decideSolverStep(`${prompt}\n\n本次只提出下一步决策和模块检索词，不生成 DAG。${feedback}`, emit), signal, report);
    if (decision.action === "stop") throw new Error(`Agent 暂停尝试：${decision.nextGoal}`);
    if (decision.repairs.some((r) => !previous?.nodes.some((n) => n.key === r.node))) {
      if (searchAttempt === 1) throw new Error("诊断引用了当前图不存在的节点，未执行修改");
      feedback = `\n上次诊断引用了不存在的节点。当前可修复节点 key：${JSON.stringify(previous?.nodes.map((n) => n.key) ?? [])}，请修正。`;
      continue;
    }
    const list = Object.values(descriptors).filter((d) => isSolverModule(d.id));
    const searches = decision.queries.map((query) => ({ query,
      matches: searchNodes(list, query).slice(0, 8).map((d) => ({ id: d.id, name: d.displayName })) }));
    const baseline = ["text_input", "file_import", "image_input", "text_output", ...(previous?.nodes.map((n) => n.descriptorId) ?? [])];
    const catalogIds = [...new Set([...baseline, ...searches.flatMap((s) => s.matches.map((m) => m.id))])].filter((id) => isSolverModule(id) && descriptors[id]);
    planning = { decision, searches, catalogIds };
    onPlanning(planning);
    searches.forEach((s) => report(`查找「${s.query}」：${s.matches.length ? s.matches.map((m) => `${m.name} (${m.id})`).join("、") : "未找到对应模块"}`));
    if (searches.some((s) => s.matches.length)) break;
    feedback = `\n实际模块检索全部未命中：${JSON.stringify(searches)}。换用简短中文名或英文算法名重新检索；不支持时 action=stop 并说明缺口。`;
    if (searchAttempt === 1) throw new Error(`未找到可用模块：${decision.queries.join("、")}。已保留流程和证据。`);
  }
  if (!planning) throw new Error("缺少下一步决策");
  let correction = "";
  for (let retry = 0; retry < 2; retry++) {
    const instructions = `${prompt}\n\n已作出的下一步决策和本机真实检索结果：${JSON.stringify(planning)}\n只能使用检索命中或当前图已有的模块，按决策修改当前图、继续拼接，保持现有有效节点 key。\n${correction}`;
    const generated = await agentRequest((emit) => api.generateWorkflow(instructions, emit, planning!.catalogIds), signal, report);
    try {
      if (generated.nodes.some((n) => !planning!.catalogIds.includes(n.descriptorId))) throw new Error("出现尚未检索确认的模块");
      return { graph: prepareSolvePlan(generated, descriptors, options), planning };
    } catch (e) {
      correction = `拟议计划未通过校验：${e instanceof Error ? e.message : String(e)}；请修正。`;
      report(correction);
      if (retry === 1) throw e;
    }
  }
  throw new Error("未生成可执行计划");
}
