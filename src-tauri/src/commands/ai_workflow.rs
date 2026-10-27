//! AI workflow generation. Given a natural-language task, ask the configured LLM
//! to assemble a node graph from the registry's catalog, validate it against the
//! real node signatures, auto-lay-it-out, and return a Template-shaped graph the
//! frontend reviews before confirming graph creation and execution.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::State;

use misclab_core::ai::{self, ModelConfig};
use misclab_core::cancel::CancellationToken;
use misclab_core::graph::port::PortType;
use misclab_core::node::descriptor::{NodeDescriptor, ParamSpec, ParamWidget, PortSpec};
use misclab_core::node::registry::NodeRegistry;

use super::graph::ProgressMsg;
use crate::error::AppError;
use crate::state::{combined_registry_from, AppState};

#[cfg(test)]
#[path = "ai_live_test.rs"]
mod live_test;

// ---- output (matches the frontend Template node/edge shape) ----------------

#[derive(Serialize)]
pub(crate) struct Pos {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GenNode {
    pub(crate) key: String,
    pub(crate) descriptor_id: String,
    pub(crate) params: serde_json::Value,
    pub(crate) position: Pos,
}

#[derive(Serialize)]
pub(crate) struct GenRef {
    pub(crate) node: String,
    pub(crate) port: String,
}

#[derive(Serialize)]
pub(crate) struct GenEdge {
    pub(crate) from: GenRef,
    pub(crate) to: GenRef,
}

#[derive(Serialize)]
pub struct GeneratedGraph {
    pub(crate) nodes: Vec<GenNode>,
    pub(crate) edges: Vec<GenEdge>,
    pub(crate) notes: String,
    pub(crate) checks: Vec<PlanCheck>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PlanCheck {
    node: String,
    port: String,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expected: Option<String>,
}

#[derive(Serialize)]
pub struct AiTextResult {
    text: String,
}

// ---- what we parse back from the LLM ---------------------------------------

#[derive(Deserialize)]
struct LlmNode {
    key: String,
    #[serde(rename = "type")]
    descriptor_id: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Deserialize)]
struct LlmEdge {
    from: String,
    to: String,
}

#[derive(Deserialize)]
struct LlmGraph {
    #[serde(default)]
    nodes: Vec<LlmNode>,
    #[serde(default)]
    edges: Vec<LlmEdge>,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    checks: Vec<PlanCheck>,
}

// ---- catalog ---------------------------------------------------------------

pub(crate) fn pt_str(t: PortType) -> &'static str {
    match t {
        PortType::Any => "any",
        PortType::Text => "text",
        PortType::Number => "number",
        PortType::Bool => "bool",
        PortType::Json => "json",
        PortType::StringList => "stringList",
        PortType::Candidates => "candidates",
        PortType::Bytes => "bytes",
        PortType::Artifact => "artifact",
        PortType::Image => "image",
        PortType::Fingerprint => "fingerprint",
    }
}

fn param_str(p: &ParamSpec) -> String {
    let widget = match &p.widget {
        ParamWidget::Select { options } => format!("{}=select[{}]", p.name, options.join("|")),
        ParamWidget::Toggle => format!("{}=toggle", p.name),
        ParamWidget::Number { min, max, step } | ParamWidget::Slider { min, max, step } => {
            format!("{}=number[{min}..{max};step={step}]", p.name)
        }
        ParamWidget::File => format!("{}=file", p.name),
        ParamWidget::Image => format!("{}=image", p.name),
        ParamWidget::Text { .. } => format!("{}=text", p.name),
    };
    format!(
        "{widget};default={};help={}",
        truncate(&p.default.to_string(), 80),
        truncate(&p.description, 80)
    )
}

fn ports_str(ports: &[PortSpec]) -> String {
    if ports.is_empty() {
        return "-".into();
    }
    ports
        .iter()
        .map(|p| {
            format!(
                "{}:{}{}",
                p.name,
                pt_str(p.port_type),
                if p.required { "!" } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn catalog_line(d: &NodeDescriptor) -> String {
    let params = if d.params.is_empty() {
        "-".to_string()
    } else {
        d.params
            .iter()
            .map(param_str)
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "{} | {} | category:{}/{} | family:{} | in:{} | out:{} | params:{}\n",
        d.id,
        d.display_name,
        d.category,
        d.subcategory_id,
        d.family,
        ports_str(&d.inputs),
        ports_str(&d.outputs),
        params
    )
}

pub(crate) fn build_catalog(ds: &[NodeDescriptor]) -> String {
    ds.iter().map(catalog_line).collect()
}

/// Catalog text for only the descriptors whose id is in `only` (used by the
/// port-suggestion command to keep the prompt small and on-topic).
pub(crate) fn build_catalog_filtered(ds: &[NodeDescriptor], only: &[&str]) -> String {
    let set: std::collections::HashSet<&str> = only.iter().copied().collect();
    ds.iter()
        .filter(|d| set.contains(d.id.as_str()))
        .map(catalog_line)
        .collect()
}

fn extract_json(s: &str) -> Option<&str> {
    let start = s.find('{')?;
    let end = s.rfind('}')?;
    (end > start).then(|| &s[start..=end])
}

pub(crate) fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n).collect::<String>())
    }
}

/// Resolve a `"key.port"` reference against known nodes; fill the default port if
/// omitted. `is_source` picks output vs input/param validation.
fn resolve(
    r: &str,
    descs: &HashMap<String, &NodeDescriptor>,
    is_source: bool,
) -> Option<(String, String)> {
    let (key, port) = match r.split_once('.') {
        Some((k, p)) => (k.trim().to_string(), Some(p.trim().to_string())),
        None => (r.trim().to_string(), None),
    };
    let d = descs.get(&key)?;
    let port = match port {
        Some(p) => {
            let ok = if is_source {
                d.outputs.iter().any(|o| o.name == p)
            } else {
                d.inputs.iter().any(|i| i.name == p) || d.params.iter().any(|pp| pp.name == p)
            };
            if ok {
                p
            } else {
                return None;
            }
        }
        None => {
            if is_source {
                d.outputs.first()?.name.clone()
            } else {
                d.inputs.first()?.name.clone()
            }
        }
    };
    Some((key, port))
}

/// Layered left-to-right layout: x by longest-path depth, y by order within a layer.
fn layout(keys: &[String], edges: &[(String, String)]) -> HashMap<String, Pos> {
    let mut level: HashMap<String, usize> = keys.iter().map(|k| (k.clone(), 0usize)).collect();
    for _ in 0..keys.len().max(1) {
        let mut changed = false;
        for (f, t) in edges {
            if let (Some(&lf), Some(&lt)) = (level.get(f), level.get(t)) {
                if lf + 1 > lt {
                    level.insert(t.clone(), lf + 1);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let mut by_level: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for k in keys {
        by_level
            .entry(*level.get(k).unwrap_or(&0))
            .or_default()
            .push(k.clone());
    }
    let mut pos = HashMap::new();
    for (lvl, ks) in by_level {
        for (i, k) in ks.into_iter().enumerate() {
            pos.insert(
                k,
                Pos {
                    x: 40.0 + lvl as f64 * 260.0,
                    y: 60.0 + i as f64 * 150.0,
                },
            );
        }
    }
    pos
}

fn generation_system(registry: &NodeRegistry, only: Option<&[String]>) -> String {
    let descriptors = registry.descriptors();
    let catalog = match only {
        Some(ids) => build_catalog_filtered(
            &descriptors,
            &ids.iter().map(String::as_str).collect::<Vec<_>>(),
        ),
        None => build_catalog(&descriptors),
    };

    format!(
        "你是 LovelyMiscLab 的流程生成器。用户会描述一个 CTF misc 处理任务，你要用下列节点搭一个数据流图(DAG)来完成它。\n\n\
【可用节点】格式: id | 名称 | 分类/分组 | 算法家族 | 输入端口 | 输出端口 | 参数\n{catalog}\n\
【规则】\n\
1. 只能使用上面出现过的节点 id。\n\
2. 用连线把上游【输出端口】连到下游【输入端口】，类型要匹配(text↔text, bytes↔bytes；any 可接任何类型)。\n\
3. 任务若从一段文本开始，用 text_input 作为源，并把该文本填进它的 text 参数；但若用户说明待处理数据已由程序单独填入，则 text_input 的 text 留空、不要重复那段数据。\n\
4. 需要展示最终文本结果时末尾接 text_output。\n\
5. select 参数只能取方括号里给出的选项之一。\n\
6. 参数也能被连线驱动：把某上游输出连到目标节点的【参数名】即可。\n\
7. 自动配置已知参数；遗漏参数使用目录默认值，数值遵守范围。连接所有带 ! 的必需输入。未知密钥、路径不能编造。notes 说明步骤、预期结果、缺少的线索及文件/网络副作用。\n\
8. checks 可选，用于运行后测试文本输出。仅当用户提供了明确结果或目标特征时添加，禁止编造预期值。格式为 {{\"node\":\"dec\",\"port\":\"text\",\"kind\":\"equals\",\"expected\":\"Hello\"}}；kind 支持 equals、contains、nonEmpty（无需 expected）。text_output 的展示端口为 value。\n\
9. 只输出一个 JSON 对象，禁止任何解释文字或 markdown 代码块。不运行节点，等待用户确认。\n\n\
【输出格式】\n\
{{\"nodes\":[{{\"key\":\"in\",\"type\":\"text_input\",\"params\":{{\"text\":\"...\"}}}},{{\"key\":\"dec\",\"type\":\"base64_decode\"}},{{\"key\":\"out\",\"type\":\"text_output\"}}],\"edges\":[{{\"from\":\"in.text\",\"to\":\"dec.text\"}},{{\"from\":\"dec.text\",\"to\":\"out.text\"}}],\"notes\":\"一句话思路\"}}"
    )
}

#[cfg(feature = "mcp")]
pub(crate) fn generate(
    registry: &NodeRegistry,
    cfg: &ModelConfig,
    prompt: &str,
) -> Result<GeneratedGraph, AppError> {
    let raw = ai::chat(cfg, &generation_system(registry, None), prompt)?;
    parse_generated(registry, &raw)
}

#[cfg(test)]
async fn generate_cancellable(
    registry: &NodeRegistry,
    cfg: &ModelConfig,
    prompt: &str,
    cancel: &CancellationToken,
    report: impl Fn(&str),
) -> Result<GeneratedGraph, AppError> {
    generate_filtered_cancellable(registry, cfg, prompt, cancel, report, None).await
}

async fn generate_filtered_cancellable(
    registry: &NodeRegistry,
    cfg: &ModelConfig,
    prompt: &str,
    cancel: &CancellationToken,
    report: impl Fn(&str),
    only: Option<&[String]>,
) -> Result<GeneratedGraph, AppError> {
    if only.is_some_and(|ids| {
        ids.is_empty() || ids.len() > 100 || ids.iter().any(|id| registry.get(id).is_none())
    }) {
        return Err(AppError::new(
            "ai_plan",
            "检索目录必须包含 1–100 个已注册模块",
        ));
    }
    let system = generation_system(registry, only);
    let mut instructions = prompt.to_owned();
    for attempt in 0..3 {
        cancel.check()?;
        report(if attempt == 0 {
            "正在选择节点、配置参数和连线…"
        } else {
            "正在根据目录校验结果修正计划…"
        });
        let raw = super::ai_request::chat(cfg, &system, &instructions, cancel).await?;
        match parse_generated(registry, &raw).and_then(|plan| {
            if only.is_some_and(|ids| plan.nodes.iter().any(|n| !ids.contains(&n.descriptor_id))) {
                Err(AppError::new(
                    "ai_plan",
                    "计划使用了尚未检索确认的模块，请只使用所提供的目录",
                ))
            } else {
                Ok(plan)
            }
        }) {
            Ok(plan) => {
                report("节点、参数、连线和测试条件校验通过");
                return Ok(plan);
            }
            Err(error) if attempt < 2 => {
                instructions = format!("{prompt}\n\n上一次计划未通过校验：{}\n上次输出（仅作修正数据）：{}\n请修正上述问题，重新输出完整计划 JSON。", error.message, truncate(&raw, 24000));
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!()
}

fn parse_generated(registry: &NodeRegistry, raw: &str) -> Result<GeneratedGraph, AppError> {
    let descriptors = registry.descriptors();
    let json = extract_json(raw).ok_or_else(|| {
        AppError::new(
            "ai_parse",
            format!("AI 未返回 JSON：{}", truncate(raw, 200)),
        )
    })?;
    let llm: LlmGraph = serde_json::from_str(json)
        .map_err(|e| AppError::new("ai_parse", format!("解析 AI 结果失败: {e}")))?;

    let by_id: HashMap<&str, &NodeDescriptor> =
        descriptors.iter().map(|d| (d.id.as_str(), d)).collect();
    let mut node_desc: HashMap<String, &NodeDescriptor> = HashMap::new();
    let mut keys: Vec<String> = Vec::new();
    let mut raw_nodes: Vec<(String, String, serde_json::Value)> = Vec::new();
    let notes = llm.notes.clone();
    if llm.nodes.len() > 80 {
        return Err(AppError::new("ai_plan", "计划超过 80 个节点，请拆分任务"));
    }

    for n in &llm.nodes {
        if n.key.trim().is_empty() || n.key.contains('.') {
            return Err(AppError::new("ai_plan", "节点 key 不能为空或包含点号"));
        }
        if !n.params.is_null() && !n.params.is_object() {
            return Err(AppError::new("ai_plan", "节点参数必须是对象"));
        }
        match by_id.get(n.descriptor_id.as_str()) {
            Some(d) if !node_desc.contains_key(&n.key) => {
                node_desc.insert(n.key.clone(), d);
                keys.push(n.key.clone());
                let params = if n.params.is_object() {
                    n.params.clone()
                } else {
                    serde_json::json!({})
                };
                raw_nodes.push((n.key.clone(), n.descriptor_id.clone(), params));
            }
            Some(_) => return Err(AppError::new("ai_plan", format!("重复节点：{}", n.key))),
            None => {
                return Err(AppError::new(
                    "ai_plan",
                    format!("未知节点：{}", n.descriptor_id),
                ))
            }
        }
    }
    if raw_nodes.is_empty() {
        return Err(AppError::new(
            "ai_empty",
            "AI 未生成任何有效节点，请换个描述再试。",
        ));
    }

    let mut edge_pairs: Vec<(String, String)> = Vec::new();
    let mut edges: Vec<GenEdge> = Vec::new();
    for e in &llm.edges {
        if let (Some((fk, fp)), Some((tk, tp))) = (
            resolve(&e.from, &node_desc, true),
            resolve(&e.to, &node_desc, false),
        ) {
            edge_pairs.push((fk.clone(), tk.clone()));
            edges.push(GenEdge {
                from: GenRef { node: fk, port: fp },
                to: GenRef { node: tk, port: tp },
            });
        } else {
            return Err(AppError::new(
                "ai_plan",
                format!("无效连线：{} → {}", e.from, e.to),
            ));
        }
    }

    let pos = layout(&keys, &edge_pairs);
    let nodes = raw_nodes
        .into_iter()
        .map(|(key, descriptor_id, params)| {
            let p = pos
                .get(&key)
                .map(|p| Pos { x: p.x, y: p.y })
                .unwrap_or(Pos { x: 40.0, y: 60.0 });
            GenNode {
                key,
                descriptor_id,
                params,
                position: p,
            }
        })
        .collect();

    let result = GeneratedGraph {
        nodes,
        edges,
        notes,
        checks: llm.checks,
    };
    validate_plan(registry, &result)?;
    Ok(result)
}

fn validate_plan(registry: &NodeRegistry, plan: &GeneratedGraph) -> Result<(), AppError> {
    use misclab_core::graph::{
        compute::ComputeGraph,
        model::{Edge, NodeInstance, PortRef, SerializedGraph},
    };
    let lookup = |key: &str| {
        plan.nodes
            .iter()
            .find(|n| n.key == key)
            .and_then(|n| registry.get(&n.descriptor_id))
            .map(|e| &e.descriptor)
    };
    if plan.checks.len() > 20 {
        return Err(AppError::new("ai_plan", "测试条件最多 20 项"));
    }
    for check in &plan.checks {
        let valid_port = lookup(&check.node).is_some_and(|d| {
            (d.id == "text_output" && check.port == "value")
                || d.outputs.iter().any(|p| {
                    p.name == check.port && matches!(p.port_type, PortType::Text | PortType::Any)
                })
        });
        let valid_condition = match check.kind.as_str() {
            "equals" | "contains" => check
                .expected
                .as_ref()
                .is_some_and(|s| s.chars().count() <= 4096),
            "nonEmpty" => true,
            _ => false,
        };
        if !valid_port || !valid_condition {
            return Err(AppError::new(
                "ai_plan",
                format!("测试条件无效：{}.{}", check.node, check.port),
            ));
        }
    }
    for node in &plan.nodes {
        let d = lookup(&node.key).ok_or_else(|| AppError::new("ai_plan", "未知节点"))?;
        for (name, value) in node.params.as_object().into_iter().flatten() {
            let spec = d
                .params
                .iter()
                .find(|p| p.name == *name)
                .ok_or_else(|| AppError::new("ai_plan", format!("{} 没有参数 {name}", d.id)))?;
            let valid = match &spec.widget {
                ParamWidget::Select { options } => value
                    .as_str()
                    .is_some_and(|s| options.iter().any(|o| o == s)),
                ParamWidget::Toggle => value.is_boolean(),
                ParamWidget::Number { min, max, .. } | ParamWidget::Slider { min, max, .. } => {
                    value
                        .as_f64()
                        .is_some_and(|n| n.is_finite() && n >= *min && n <= *max)
                }
                _ => value.is_string(),
            };
            if !valid {
                return Err(AppError::new(
                    "ai_plan",
                    format!("{} 参数 {name} 的值无效", d.id),
                ));
            }
        }
        for port in &d.inputs {
            if port.required
                && !plan
                    .edges
                    .iter()
                    .any(|e| e.to.node == node.key && e.to.port == port.name)
            {
                return Err(AppError::new(
                    "ai_plan",
                    format!("{} 必需输入 {} 未连接", node.key, port.name),
                ));
            }
        }
    }
    for edge in &plan.edges {
        let source = lookup(&edge.from.node)
            .and_then(|d| d.outputs.iter().find(|p| p.name == edge.from.port))
            .map(|p| p.port_type);
        let target = lookup(&edge.to.node).and_then(|d| {
            d.inputs
                .iter()
                .find(|p| p.name == edge.to.port)
                .map(|p| p.port_type)
                .or_else(|| {
                    d.params
                        .iter()
                        .find(|p| p.name == edge.to.port)
                        .map(|p| param_port_type(&p.widget))
                })
        });
        if !matches!((source, target), (Some(s), Some(t)) if t.accepts(s)) {
            return Err(AppError::new("ai_plan", "计划含无效端口或不兼容的连线"));
        }
    }
    let graph = SerializedGraph {
        nodes: plan
            .nodes
            .iter()
            .map(|n| NodeInstance {
                id: n.key.clone(),
                descriptor_id: n.descriptor_id.clone(),
                params: n.params.clone(),
                position: (n.position.x as f32, n.position.y as f32),
            })
            .collect(),
        edges: plan
            .edges
            .iter()
            .map(|e| Edge {
                from: PortRef {
                    node: e.from.node.clone(),
                    port: e.from.port.clone(),
                },
                to: PortRef {
                    node: e.to.node.clone(),
                    port: e.to.port.clone(),
                },
            })
            .collect(),
    };
    ComputeGraph::from_serialized(&graph)?;
    Ok(())
}

fn graph_summary(
    registry: &NodeRegistry,
    graph: &misclab_core::graph::model::SerializedGraph,
) -> String {
    let descriptors = registry.descriptors();
    let by_id: HashMap<&str, &NodeDescriptor> =
        descriptors.iter().map(|d| (d.id.as_str(), d)).collect();
    let mut out = String::new();
    out.push_str("节点:\n");
    for n in &graph.nodes {
        let name = by_id
            .get(n.descriptor_id.as_str())
            .map(|d| d.display_name.as_str())
            .unwrap_or(n.descriptor_id.as_str());
        out.push_str(&format!(
            "- {}: {} ({}) params={}\n",
            n.id, name, n.descriptor_id, n.params
        ));
    }
    out.push_str("连线:\n");
    for e in &graph.edges {
        out.push_str(&format!(
            "- {}.{} -> {}.{}\n",
            e.from.node, e.from.port, e.to.node, e.to.port
        ));
    }
    out
}

#[tauri::command]
pub async fn generate_workflow(
    state: State<'_, AppState>,
    prompt: String,
    on_event: Channel<ProgressMsg>,
    catalog_ids: Option<Vec<String>>,
) -> Result<GeneratedGraph, AppError> {
    if prompt.trim().is_empty() {
        return Err(AppError::new("ai_input", "请先描述你要做的任务。"));
    }
    let cfg = state
        .settings
        .lock()
        .expect("settings mutex")
        .ai
        .llm
        .clone();
    let registry = workflow_registry(&state);
    let cancel = CancellationToken::new();
    let job = state.jobs.start(cancel.clone());
    if on_event
        .send(ProgressMsg::JobStarted { job: job.clone() })
        .is_err()
    {
        cancel.cancel();
    }
    let result = generate_filtered_cancellable(
        &registry,
        &cfg,
        &prompt,
        &cancel,
        |message| {
            if on_event
                .send(ProgressMsg::Log {
                    node: None,
                    level: "info".into(),
                    message: message.into(),
                })
                .is_err()
            {
                cancel.cancel();
            }
        },
        catalog_ids.as_deref(),
    )
    .await;
    state.jobs.finish(&job);
    result
}

fn workflow_registry(state: &AppState) -> NodeRegistry {
    combined_registry_from(
        &state.registry,
        &state.composites.lock().expect("composites mutex"),
        &state.scripts.lock().expect("scripts mutex"),
    )
}

#[tauri::command]
pub async fn explain_workflow(
    state: State<'_, AppState>,
    graph: misclab_core::graph::model::SerializedGraph,
    prompt: String,
    on_event: Channel<ProgressMsg>,
) -> Result<AiTextResult, AppError> {
    if graph.nodes.is_empty() {
        return Err(AppError::new("ai_input", "当前画布为空，无法解释流程。"));
    }
    let cfg = state
        .settings
        .lock()
        .expect("settings mutex")
        .ai
        .llm
        .clone();
    let registry = workflow_registry(&state);
    let cancel = CancellationToken::new();
    let job = state.jobs.start(cancel.clone());
    if on_event
        .send(ProgressMsg::JobStarted { job: job.clone() })
        .is_err()
    {
        cancel.cancel();
    }
    let result = async {
        let summary = graph_summary(&registry, &graph);
        let system = "你是 LovelyMiscLab 的工作流讲解助手。请用中文解释节点图的数据流、关键参数、可能的失败点和下一步优化建议。回答要具体、简洁、可执行。";
        let user = if prompt.trim().is_empty() {
            format!("请解释这个工作流:\n{summary}")
        } else {
            format!("用户关注点: {}\n\n工作流:\n{summary}", prompt.trim())
        };
        let text = super::ai_request::chat(&cfg, system, &user, &cancel).await?;
        Ok(AiTextResult { text })
    }.await;
    state.jobs.finish(&job);
    result
}

// ---- port-hover next-node suggestions ---------------------------------------

/// The port type a param exposes when driven by a connection — mirrors the
/// frontend `paramPortType`.
pub(crate) fn param_port_type(w: &ParamWidget) -> PortType {
    match w {
        ParamWidget::Number { .. } | ParamWidget::Slider { .. } => PortType::Number,
        ParamWidget::Toggle => PortType::Bool,
        ParamWidget::Image => PortType::Image,
        _ => PortType::Text, // text / select / file
    }
}

/// Is `d` a sensible neighbour for a port of type `pt`? `dir_out` = the hovered
/// port is an output (we want a downstream consumer); otherwise it's an input
/// (we want an upstream producer).
fn port_compatible(d: &NodeDescriptor, pt: PortType, dir_out: bool) -> bool {
    if dir_out {
        d.inputs.iter().any(|i| i.port_type.accepts(pt))
            || d.params
                .iter()
                .any(|p| param_port_type(&p.widget).accepts(pt))
    } else {
        d.outputs.iter().any(|o| pt.accepts(o.port_type))
    }
}

fn extract_json_array(s: &str) -> Option<&str> {
    let start = s.find('[')?;
    let end = s.rfind(']')?;
    (end > start).then(|| &s[start..=end])
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestCtx {
    descriptor_id: String,
    port: String,
    /// "in" or "out".
    direction: String,
    /// Serialized `PortType`, e.g. "text".
    port_type: String,
    #[serde(default)]
    hint: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    descriptor_id: String,
    #[serde(default)]
    reason: String,
}

fn suggest(
    registry: &NodeRegistry,
    cfg: &ModelConfig,
    ctx: &SuggestCtx,
) -> Result<Vec<Suggestion>, AppError> {
    let pt: PortType = serde_json::from_value(serde_json::Value::String(ctx.port_type.clone()))
        .unwrap_or(PortType::Any);
    let dir_out = ctx.direction != "in";
    let descriptors = registry.descriptors();
    let compatible_ids: Vec<&str> = descriptors
        .iter()
        .filter(|d| port_compatible(d, pt, dir_out))
        .map(|d| d.id.as_str())
        .collect();
    if compatible_ids.is_empty() {
        return Ok(vec![]);
    }
    let catalog = build_catalog_filtered(&descriptors, &compatible_ids);
    let dir_desc = if dir_out {
        "下游（消费该输出）"
    } else {
        "上游（产生该输入）"
    };
    let system = format!(
        "你是 LovelyMiscLab 的节点推荐助手。用户想在某节点的一个端口上接一个{dir_desc}节点。\n\n\
【候选节点】格式: id | 名称 | 分类/分组 | 算法家族 | 输入 | 输出 | 参数\n{catalog}\n\
【要求】\n\
1. 只能从上面候选里选，按适用度从高到低排序，返回 3-6 个。\n\
2. 每个给一句极简中文理由（≤12字）。\n\
3. 只输出 JSON 数组：[{{\"descriptorId\":\"...\",\"reason\":\"...\"}}]，禁止任何其他文字或 markdown。"
    );
    let user = match ctx.hint.as_deref().map(str::trim) {
        Some(h) if !h.is_empty() => format!(
            "当前节点 {} 的端口「{}」(类型 {})。用户想做: {}",
            ctx.descriptor_id, ctx.port, ctx.port_type, h
        ),
        _ => format!(
            "当前节点 {} 的端口「{}」(类型 {})。给出最常见有用的下一步。",
            ctx.descriptor_id, ctx.port, ctx.port_type
        ),
    };
    let raw = ai::chat(cfg, &system, &user)?;
    let json = extract_json_array(&raw).unwrap_or(raw.as_str());
    let parsed: Vec<Suggestion> = serde_json::from_str(json).unwrap_or_default();
    // Drop any id the model invented that isn't in the compatible set.
    let allow: std::collections::HashSet<&str> = compatible_ids.into_iter().collect();
    Ok(parsed
        .into_iter()
        .filter(|s| allow.contains(s.descriptor_id.as_str()))
        .collect())
}

/// Suggest compatible next/previous nodes for a hovered port. Returns `Err` when
/// the LLM isn't configured — the frontend then just keeps its static candidate
/// list.
#[tauri::command]
pub async fn suggest_next_nodes(
    state: State<'_, AppState>,
    ctx: SuggestCtx,
) -> Result<Vec<Suggestion>, AppError> {
    let cfg = state
        .settings
        .lock()
        .expect("settings mutex")
        .ai
        .llm
        .clone();
    let registry = state.registry.clone();
    tauri::async_runtime::spawn_blocking(move || suggest(&registry, &cfg, &ctx))
        .await
        .map_err(|e| AppError::new("join", e.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn valid() -> Value {
        json!({"nodes":[
            {"key":"input","type":"text_input","params":{"text":"hello"}},
            {"key":"out","type":"text_output"}
        ],"edges":[{"from":"input.text","to":"out.text"}],"notes":"显示文本"})
    }

    fn parse(value: &Value) -> Result<GeneratedGraph, AppError> {
        parse_generated(&misclab_core::nodes::default_registry(), &value.to_string())
    }

    #[test]
    fn validates_and_lays_out_a_plan_without_running_it() {
        let plan = parse(&valid()).unwrap();
        assert_eq!(plan.nodes.len(), 2);
        assert_eq!(plan.edges.len(), 1);
        assert_eq!(plan.nodes[0].params["text"], "hello");
        assert!(plan.nodes[1].position.x > plan.nodes[0].position.x);
        assert_eq!(plan.notes, "显示文本");
    }

    #[test]
    fn rejects_unknown_duplicate_and_ambiguous_node_keys() {
        for (field, value) in [
            ("type", "invented"),
            ("key", "input"),
            ("key", " "),
            ("key", "out.text"),
        ] {
            let mut plan = valid();
            plan["nodes"][1][field] = json!(value);
            assert!(parse(&plan).is_err(), "{field}: {value}");
        }
    }

    #[test]
    fn rejects_missing_required_input_unknown_port_and_duplicate_input() {
        let mut plan = valid();
        plan["edges"] = json!([]);
        assert!(parse(&plan).is_err());
        let mut plan = valid();
        plan["edges"][0]["from"] = json!("input.no_such_port");
        assert!(parse(&plan).is_err());
        let mut plan = valid();
        let edge = plan["edges"][0].clone();
        plan["edges"].as_array_mut().unwrap().push(edge);
        assert!(parse(&plan).is_err());
    }

    #[test]
    fn rejects_wrong_parameter_names_types_and_container() {
        for params in [json!({"text":42}), json!({"unknown":"x"}), json!(["text"])] {
            let mut plan = valid();
            plan["nodes"][0]["params"] = params;
            assert!(parse(&plan).is_err());
        }
    }

    #[test]
    fn rejects_cycles_and_incompatible_types() {
        let cycle = json!({"nodes":[{"key":"a","type":"base64_decode"},{"key":"b","type":"base64_decode"}],
            "edges":[{"from":"a.text","to":"b.text"},{"from":"b.text","to":"a.text"}]});
        assert!(parse(&cycle).is_err());
        let mut plan = valid();
        plan["nodes"][0] = json!({"key":"input","type":"image_input"});
        plan["edges"][0]["from"] = json!("input.image");
        assert!(parse(&plan).is_err());
    }

    #[test]
    fn accepts_connected_parameters_and_rejects_invalid_select() {
        let mut plan = valid();
        plan["nodes"][1] = json!({"key":"out","type":"text_input"});
        assert!(parse(&plan).is_ok());
        plan["nodes"][1] = json!({"key":"out","type":"hash","params":{"algorithm":"SHA256"}});
        plan["edges"][0]["to"] = json!("out.data");
        assert!(parse(&plan).is_ok());
        plan["nodes"][1]["params"]["algorithm"] = json!("definitely-invalid-choice");
        assert!(parse(&plan).is_err());
    }

    #[test]
    fn rejects_empty_and_oversized_plans() {
        assert!(parse(&json!({})).is_err());
        let nodes: Vec<_> = (0..81)
            .map(|i| json!({"key":format!("n{i}"),"type":"text_input"}))
            .collect();
        assert!(parse(&json!({"nodes":nodes})).is_err());
    }

    #[test]
    fn ai_catalog_uses_the_same_taxonomy_and_ports_as_module_library() {
        let descriptors = misclab_core::nodes::default_registry().descriptors();
        let text = build_catalog_filtered(&descriptors, &["byte_swap_encode", "base64_encode"]);
        assert!(text.contains("category:进制与数值/numeric.bits | family:byteSwap"));
        assert!(text.contains("base64_encode") && text.contains("in:text:any"));
        assert!(!text.contains("byte_swap_decode"));
    }

    #[test]
    fn checks_are_validated_and_catalog_includes_parameter_defaults_and_limits() {
        let mut value = valid();
        value["checks"] = json!([{"node":"out","port":"value","kind":"equals","expected":"hello"}]);
        let serialized = serde_json::to_value(parse(&value).unwrap()).unwrap();
        assert_eq!(serialized["checks"][0]["expected"], "hello");
        value["checks"] = json!([{"node":"out","port":"value","kind":"nonEmpty","expected":null}]);
        let serialized = serde_json::to_value(parse(&value).unwrap()).unwrap();
        assert!(serialized["checks"][0].get("expected").is_none());
        for check in [
            json!({"node":"missing","port":"value","kind":"nonEmpty"}),
            json!({"node":"out","port":"missing","kind":"equals","expected":"x"}),
            json!({"node":"out","port":"value","kind":"equals"}),
            json!({"node":"out","port":"value","kind":"invalid"}),
        ] {
            value["checks"] = json!([check]);
            assert!(parse(&value).is_err());
        }
        let registry = misclab_core::nodes::default_registry();
        let catalog = build_catalog_filtered(&registry.descriptors(), &["qr_encode"]);
        assert!(catalog.contains("default="));
        assert!(catalog.contains("step="));
    }

    #[tokio::test]
    async fn repairs_invalid_model_plan_then_executes_real_rust_graph() {
        use misclab_core::{
            graph::{executor::GraphExecutor, model::SerializedGraph},
            progress::NullSink,
        };
        let raw = json!({"nodes":[
            {"key":"source","type":"text_input","params":{"text":"SGVsbG8="}},
            {"key":"decode","type":"base64_decode"}, {"key":"out","type":"text_output"}
        ], "edges":[{"from":"source.text","to":"decode.text"},{"from":"decode.text","to":"out.text"}],
        "checks":[{"node":"out","port":"value","kind":"equals","expected":"Hello"}]}).to_string();
        let (cfg, _, server) = super::super::ai_request::tests::server(
            vec![
                "{\"nodes\":[{\"key\":\"bad\",\"type\":\"invented\"}]}".into(),
                raw,
            ],
            std::time::Duration::ZERO,
        );
        let registry = misclab_core::nodes::default_registry();
        let plan = generate_cancellable(
            &registry,
            &cfg,
            "Base64 解码，预期 Hello",
            &CancellationToken::new(),
            |_| {},
        )
        .await
        .unwrap();
        let calls = server.join().unwrap();
        assert_eq!(calls.len(), 2);
        assert!(calls[1]["messages"][1]["content"]
            .as_str()
            .unwrap()
            .contains("未知节点"));
        let graph: SerializedGraph = serde_json::from_value(json!({
            "nodes": plan.nodes.iter().map(|n| json!({"id":n.key,"descriptorId":n.descriptor_id,"params":n.params,"position":[n.position.x,n.position.y]})).collect::<Vec<_>>(),
            "edges":plan.edges
        })).unwrap();
        let outputs = GraphExecutor::new(&registry, &graph)
            .unwrap()
            .run(&NullSink, &CancellationToken::new())
            .unwrap();
        assert!(
            matches!(&outputs["out"]["value"], misclab_core::graph::port::PortValue::Text(text) if text == "Hello")
        );
    }

    #[tokio::test]
    async fn invalid_model_plans_stop_after_two_corrections() {
        let (cfg, _, server) = super::super::ai_request::tests::server(
            vec!["{}".into(); 3],
            std::time::Duration::ZERO,
        );
        let result = generate_cancellable(
            &misclab_core::nodes::default_registry(),
            &cfg,
            "test",
            &CancellationToken::new(),
            |_| {},
        )
        .await;
        assert!(result.is_err());
        assert_eq!(server.join().unwrap().len(), 3);
    }
}
