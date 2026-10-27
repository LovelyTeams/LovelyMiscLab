import { api, type GeneratedGraph } from "@/lib/bindings";
import type { NodeDescriptor } from "@/lib/types";
import { agentRequest, type AiRequestOptions } from "./agentRequest";
import { findSolverModules, solverModuleIndex } from "./agentModuleSearch";
import { isSolverModule, prepareSolvePlan, solvePrompt, type SolveAttempt, type SolveOptions, type SolvePlanning } from "./agentSolve";

/** Decide first, query the same catalog as the module library, then provide only
 * real matching signatures plus the current graph's modules to the planner. */
export async function planSolveRound(options: SolveOptions, attempts: SolveAttempt[], previous: GeneratedGraph | undefined,
  descriptors: Record<string, NodeDescriptor>, signal: AbortSignal, report: (message: string) => void,
  onPlanning: (planning: SolvePlanning) => void = () => {}, requestOptions: AiRequestOptions = {}) {
  const prompt = solvePrompt(options, attempts);
  const list = Object.values(descriptors).filter((d) => isSolverModule(d.id));
  const index = solverModuleIndex(list);
  let planning: SolvePlanning | undefined, feedback = "";
  const searchesSoFar = new Map<string, SolvePlanning["searches"][number]>();
  for (let searchAttempt = 0; searchAttempt < 3; searchAttempt++) {
    report("分析实际证据，判断需要修复或继续处理的节点…");
    const decision = await agentRequest((emit) => api.decideSolverStep(`${prompt}\n\n本次只提出下一步决策和模块检索词，不生成 DAG。\n本机可用模块索引（ID=名称，已按用途分组）：\n${index}\n优先用上面真实 ID 检索；没有单个专用模块时考虑用多个已有模块组合。每项 queries 表示一个需要的能力。${feedback}`, emit), signal, report, requestOptions);
    if (decision.action === "stop") throw new Error(`Agent 暂停尝试：${decision.nextGoal}`);
    if (decision.repairs.some((r) => !previous?.nodes.some((n) => n.key === r.node))) {
      if (searchAttempt === 2) throw new Error("诊断引用了当前图不存在的节点，未执行修改");
      feedback = `\n上次诊断引用了不存在的节点。当前可修复节点 key：${JSON.stringify(previous?.nodes.map((n) => n.key) ?? [])}，请修正。`;
      continue;
    }
    const searches = [...new Set(decision.queries)].map((query) => findSolverModules(list, query));
    searches.forEach((s) => searchesSoFar.set(s.query, s));
    const baseline = ["text_input", "file_import", "image_input", "text_output", ...(previous?.nodes.map((n) => n.descriptorId) ?? [])];
    const catalogIds = [...new Set([...baseline, ...searches.flatMap((s) => s.matches.map((m) => m.id)),
      ...[...searchesSoFar.values()].flatMap((s) => s.matches.map((m) => m.id))])].filter((id) => isSolverModule(id) && descriptors[id]).slice(0, 100);
    planning = { decision, searches, searchHistory: [...searchesSoFar.values()], catalogIds };
    onPlanning(planning);
    searches.forEach((s) => report(`查找「${s.query}」：${s.matches.length ? s.matches.map((m) => `${m.name} (${m.id})`).join("、") : `该检索词未命中${s.suggestions?.length ? `，相关模块：${s.suggestions.map((m) => `${m.name} (${m.id})`).join("、")}` : "，将核对分类目录并改换检索词"}`}`));
    const missing = searches.filter((s) => !s.matches.length);
    if (!missing.length && searches.length) break;
    feedback = `\n第 ${searchAttempt + 1} 次检索结果：${JSON.stringify(searches)}。未命中词：${missing.map((s) => s.query).join("、")}。请对照本机索引重新决策，换成真实 ID，或将需求拆成多个已有模块；已命中的能力仍可保留。部分关键词命中不表示其他能力也存在。不能实现时 action=stop，说明具体缺口。`;
    if (searchAttempt === 2) throw new Error(`未找到可用模块：${missing.map((s) => s.query).join("、") || "检索词为空"}。已核对真实目录并尝试改写/拆分检索，当前图和证据保留；可补充处理方法后重试。`);
    report(`检索调整（${searchAttempt + 1}/2）：核对真实目录，改用模块 ID 或组合已有能力…`);
  }
  if (!planning) throw new Error("缺少下一步决策");
  let correction = "";
  for (let retry = 0; retry < 2; retry++) {
    const instructions = `${prompt}\n\n已作出的下一步决策和本机真实检索结果：${JSON.stringify(planning)}\n只能使用 catalogIds 中的真实模块，未命中历史和相关建议不是已验证能力；按决策修改当前图、继续拼接，保持现有有效节点 key。\n${correction}`;
    const generated = await agentRequest((emit) => api.generateWorkflow(instructions, emit, planning!.catalogIds), signal, report, requestOptions);
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
