import type { PortSpec, PortType, PortValue } from "./types";

export interface InputDraft { text: string; format?: string; value?: PortValue; enabled?: boolean }
export const defaultFormat = (type: PortType) => type === "any" ? "text" : type === "bytes" ? "hex" : type;

export function draftFromValue(value: PortValue): InputDraft {
  if (value.type === "none") return { text: "", format: "text", enabled: true };
  const text = value.type === "bytes" ? value.value.map((b) => b.toString(16).padStart(2, "0")).join(" ") :
    ["json", "fingerprint", "stringList", "candidates"].includes(value.type) ? JSON.stringify(value.value, null, 2) : String(value.value);
  return { text, format: value.type === "bytes" ? "hex" : value.type, enabled: true };
}

/** Convert the editor representation without implicit binary/text coercion. */
export function parseInput(port: PortSpec, draft?: InputDraft): PortValue | undefined {
  if (!port.required && (!draft || draft.enabled === false || draft.enabled !== true && !draft.text && !draft.value)) return undefined;
  const format = draft?.format ?? defaultFormat(port.type), text = draft?.text ?? "";
  if (port.type !== "any" && port.type !== "bytes" && format !== port.type) throw new Error(`${port.label}：输入格式与端口类型不匹配`);
  if (draft?.value) {
    if (port.type !== "any" && draft.value.type !== port.type) throw new Error(`${port.label}：文件类型与端口不匹配`);
    return draft.value;
  }
  const error = (message: string): never => { throw new Error(`${port.label}：${message}`); };
  if (port.type === "bytes" || port.type === "any" && ["hex", "base64", "utf8"].includes(format)) {
    let bytes: number[];
    if (format === "utf8") bytes = Array.from(new TextEncoder().encode(text));
    else if (format === "hex") {
      const s = text.replace(/\s/g, "");
      if (!/^(?:[0-9a-f]{2})*$/i.test(s)) error("Hex 需要完整的两位字节");
      bytes = (s.match(/../g) ?? []).map((s) => parseInt(s, 16));
    } else if (format === "base64") {
      const s = text.replace(/\s/g, "");
      if (!/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(s)) error("Base64 格式或填充无效");
      bytes = Array.from(atob(s), (c) => c.charCodeAt(0));
    } else return error("请选择文件或输入字节");
    return { type: "bytes", value: bytes };
  }
  switch (format) {
    case "text": return { type: "text", value: text };
    case "number": {
      const value = Number(text);
      if (!text.trim() || !Number.isFinite(value)) return error("需要有限数值");
      return { type: "number", value };
    }
    case "bool":
      if (text && text !== "true" && text !== "false") return error("需要 true 或 false");
      return { type: "bool", value: text === "true" };
    case "json": case "fingerprint": {
      let value: unknown;
      try { value = JSON.parse(text); } catch { return error("JSON 无效"); }
      if (format === "fingerprint" && (!value || typeof value !== "object" || Array.isArray(value))) return error("需要指纹对象");
      return { type: format, value };
    }
    case "stringList": {
      if (text.trimStart().startsWith("[")) {
        let value: unknown;
        try { value = JSON.parse(text); } catch { return error("字符串数组 JSON 无效"); }
        if (!Array.isArray(value) || value.some((s) => typeof s !== "string")) return error("需要字符串数组");
        return { type: "stringList", value };
      }
      return { type: "stringList", value: text ? text.split(/\r?\n/) : [] };
    }
    case "candidates": {
      let value: unknown;
      try { value = JSON.parse(text); } catch { return error("候选列表 JSON 无效"); }
      if (!Array.isArray(value) || value.some((v) => !v || typeof v.text !== "string" || typeof v.score !== "number" || !Number.isFinite(v.score) || v.note !== undefined && typeof v.note !== "string")) return error("每个候选需要 text 和有限的 score");
      return { type: "candidates", value };
    }
    case "artifact":
      if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(text.trim())) return error("需要有效的文件产物 UUID");
      return { type: "artifact", value: text.trim() };
    case "image":
      if (!/^data:image\/[a-z0-9.+-]+;base64,/i.test(text)) return error("请选择图片或输入图片 data URL");
      return { type: "image", value: text };
    default: return error("未知输入格式");
  }
}
export function parseInputs(ports: PortSpec[], drafts: Record<string, InputDraft>): Record<string, PortValue> {
  const out: Record<string, PortValue> = {};
  for (const port of ports) { const v = parseInput(port, drafts[port.name]); if (v) out[port.name] = v; }
  return out;
}
