//! Rison / O-Rison / A-Rison / URI variants, pure Rust recursive descent.
use super::{
    codec_support::{document_text, help},
    prelude::*,
    serial_support,
    transport_codecs::err,
};
use serde_json::{json, Map, Value};

const FORBIDDEN: &str = " '!:(),*@$";
const START: &str = "-0123456789";
const LIMIT: usize = 1024 * 1024;
const DEPTH: usize = 64;

fn quote(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .enumerate()
            .all(|(i, c)| !FORBIDDEN.contains(c) && (i != 0 || !START.contains(c)))
    {
        return s.into();
    }
    format!("'{}'", s.replace('!', "!!").replace('\'', "!'"))
}
fn encode(v: &Value, depth: usize, ctx: &NodeCtx) -> Result<String, CoreError> {
    ctx.cancel.check()?;
    if depth > DEPTH {
        return Err(err("Rison 嵌套深度超过 64"));
    }
    Ok(match v {
        Value::Null => "!n".into(),
        Value::Bool(b) => if *b { "!t" } else { "!f" }.into(),
        Value::Number(n) => n
            .as_f64()
            .filter(|n| n.is_finite())
            .map(|n| ryu_js::Buffer::new().format(n).replace('+', ""))
            .unwrap_or("!n".into()),
        Value::String(s) => quote(s),
        Value::Array(a) => format!(
            "!({})",
            a.iter()
                .map(|v| encode(v, depth + 1, ctx))
                .collect::<Result<Vec<_>, _>>()?
                .join(",")
        ),
        Value::Object(o) => {
            let mut keys: Vec<_> = o.keys().collect();
            keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            let items = keys
                .into_iter()
                .map(|k| Ok(format!("{}:{}", quote(k), encode(&o[k], depth + 1, ctx)?)))
                .collect::<Result<Vec<_>, CoreError>>()?;
            format!("({})", items.join(","))
        }
    })
}
fn uri_quote(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-~!*()_.',:@$/".contains(&b) {
            out.push(b as char);
        } else if b == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
fn uri_unquote(s: &str) -> String {
    let mut out = String::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let start = i;
        let mut chunk = Vec::new();
        while i + 2 < b.len()
            && b[i] == b'%'
            && b[i + 1].is_ascii_hexdigit()
            && b[i + 2].is_ascii_hexdigit()
        {
            chunk.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
            i += 3;
        }
        if i != start {
            match std::str::from_utf8(&chunk) {
                Ok(text) => out.push_str(text.strip_prefix('\u{feff}').unwrap_or(text)),
                Err(_) => out.extend(chunk.into_iter().map(char::from)),
            }
        } else {
            let ch = s[i..].chars().next().unwrap();
            out.push(if ch == '+' { ' ' } else { ch });
            i += ch.len_utf8();
        }
    }
    out
}

fn property_key(value: &Value) -> String {
    match value {
        Value::Number(_) => super::structured_text::ecma_json(value, false, 0),
        Value::Array(items) => items
            .iter()
            .map(|v| {
                if v.is_null() {
                    String::new()
                } else {
                    property_key(v)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ => serial_support::string_value(value),
    }
}

struct Parser<'a, 'ctx> {
    s: &'a str,
    pos: usize,
    nodes: usize,
    ctx: &'ctx NodeCtx<'ctx>,
}
impl Parser<'_, '_> {
    fn peek(&self) -> Option<char> {
        self.s[self.pos..].chars().next()
    }
    fn take(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }
    fn error(&self, s: &str) -> CoreError {
        err(format!("Rison：{s}（字节位置 {}）", self.pos))
    }
    fn expect(&mut self, ch: char) -> Result<(), CoreError> {
        if self.take() == Some(ch) {
            Ok(())
        } else {
            Err(self.error(&format!("需要 {ch}")))
        }
    }
    fn value(&mut self, depth: usize) -> Result<Value, CoreError> {
        self.ctx.cancel.check()?;
        self.nodes += 1;
        if depth > DEPTH || self.nodes > 20000 {
            return Err(self.error("嵌套深度或元素数量超限（64 层 / 20000 项）"));
        }
        if self.nodes.is_multiple_of(128) {
            self.ctx
                .progress(self.pos as f32 / self.s.len().max(1) as f32);
            self.ctx.cancel.check()?;
        }
        match self.peek().ok_or_else(|| self.error("表达式意外结束"))? {
            '!' => {
                self.take();
                match self.take() {
                    Some('t') => Ok(Value::Bool(true)),
                    Some('f') => Ok(Value::Bool(false)),
                    Some('n') => Ok(Value::Null),
                    Some('(') => self.array(depth),
                    _ => Err(self.error("未知 ! 字面量")),
                }
            }
            '(' => {
                self.take();
                self.object(depth)
            }
            '\'' => {
                self.take();
                let mut out = String::new();
                loop {
                    match self.take() {
                        Some('\'') => break,
                        Some('!') => match self.take() {
                            Some(c @ ('!' | '\'')) => out.push(c),
                            _ => return Err(self.error("字符串仅支持 !! 与 !' 转义")),
                        },
                        Some(c) => out.push(c),
                        None => return Err(self.error("字符串未闭合")),
                    }
                }
                Ok(Value::String(out))
            }
            '-' | '0'..='9' => {
                let start = self.pos;
                while self
                    .peek()
                    .is_some_and(|c| c.is_ascii_digit() || ".-eE".contains(c))
                {
                    self.take();
                }
                let n = self.s[start..self.pos]
                    .parse::<f64>()
                    .map_err(|_| self.error("数字无效"))?;
                if !n.is_finite() {
                    return Err(self.error("数字超出范围"));
                }
                Ok(json!(n))
            }
            c => {
                if FORBIDDEN.contains(c) || START.contains(c) {
                    return Err(self.error("非法标识符起点"));
                }
                let start = self.pos;
                while self.peek().is_some_and(|c| !FORBIDDEN.contains(c)) {
                    self.take();
                }
                Ok(Value::String(self.s[start..self.pos].into()))
            }
        }
    }
    fn array(&mut self, depth: usize) -> Result<Value, CoreError> {
        let mut out = Vec::new();
        while self.peek() != Some(')') {
            if !out.is_empty() {
                self.expect(',')?;
            }
            out.push(self.value(depth + 1)?);
        }
        self.take();
        Ok(Value::Array(out))
    }
    fn object(&mut self, depth: usize) -> Result<Value, CoreError> {
        let mut out = Map::new();
        let mut first = true;
        while self.peek() != Some(')') {
            if !first {
                self.expect(',')?;
            }
            first = false;
            let key = property_key(&self.value(depth + 1)?);
            self.expect(':')?;
            // Keys are inert data, including __proto__; no JS prototype mutation.
            out.insert(key, self.value(depth + 1)?);
        }
        self.take();
        Ok(Value::Object(out))
    }
}
struct Codec(bool);
impl Node for Codec {
    fn run(
        &self,
        inputs: &PortMap,
        params: &Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let input = in_text(inputs, "text")?;
        if input.len() > LIMIT {
            return Err(err("Rison 输入最多 1 MiB"));
        }
        let input = input.trim_matches(super::numeric_support::space);
        let mode = pstr(params, "mode", "value");
        if !["value", "object", "array", "uri"].contains(&mode) {
            return Err(err("Rison 模式无效"));
        }
        if input.is_empty() {
            let mut out = out_text(String::new());
            if self.0 {
                out.insert("data".into(), PortValue::Json(Value::Null));
            }
            return Ok(out);
        }
        ctx.progress(0.05);
        ctx.cancel.check()?;
        if self.0 {
            let source = match mode {
                "object" => format!("({input})"),
                "array" => format!("!({input})"),
                "uri" => uri_unquote(input),
                _ => input.into(),
            };
            let mut parser = Parser {
                s: &source,
                pos: 0,
                nodes: 0,
                ctx,
            };
            let v = parser.value(0)?;
            if parser.pos != source.len() {
                return Err(parser.error("顶层值之后还有多余字符"));
            }
            let mut out = out_text(super::structured_text::ecma_json(&v, true, 0));
            out.insert("data".into(), PortValue::Json(v));
            Ok(out)
        } else {
            let value: Value = serde_json::from_str(input)
                .map_err(|e| err(format!("Rison 编码需要合法 JSON：{e}")))?;
            if (mode == "object" && !value.is_object()) || (mode == "array" && !value.is_array()) {
                return Err(err("Rison 顶层 JSON 类型与模式不符"));
            }
            let s = encode(&value, 0, ctx)?;
            let text = match mode {
                "object" => s[1..s.len() - 1].into(),
                "array" => s[2..s.len() - 1].into(),
                "uri" => uri_quote(&s),
                _ => s,
            };
            ctx.cancel.check()?;
            Ok(out_text(text))
        }
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for decode in [false, true] {
        let mut d = document_text(desc(if decode {"rison_decode"} else {"rison_encode"}, "data.serialization", if decode {"Rison 解码"} else {"Rison 编码"}, TEAL,
            vec![t_in()], if decode {vec![t_out(),req("data","JSON 数据",PortType::Json)]} else {vec![t_out()]},
            vec![help(ParamSpec::select("mode","模式",&["value","object","array","uri"],"value"),"value 完整值；object 省略对象括号；array 省略数组 !()；uri 对完整值做宽松 URL 引用。")]),
            "rison", "JSON ↔ Rison；对象键按 UTF-16 排序，数值遵循双精度。输入最多 1 MiB，嵌套最多 64 层。解码提供 JSON 输出；不接受孤立 UTF-16 代理。",
            if decode {"(message:中文,values:!(1,!t,!n))"} else {r#"{"message":"中文","values":[1,true,null]}"#}, json!({}));
        d.aliases
            .extend(["O-Rison", "A-Rison", "URI JSON"].map(String::from));
        reg.register(d, Arc::new(move || Arc::new(Codec(decode))));
    }
}
