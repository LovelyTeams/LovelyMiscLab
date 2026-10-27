//! Check-digit calculations and the pinned upstream BIN/IBAN reference tables.
use super::{
    codec_support::document_text,
    numeric_support::{bad, space},
    prelude::*,
};
use serde_json::{json, Value};
use std::sync::LazyLock;
static TABLES: LazyLock<Value> =
    LazyLock::new(|| serde_json::from_str(include_str!("checkdigit_tables.json")).unwrap());
#[derive(Clone, Copy)]
enum Kind {
    Luhn,
    Isbn,
    Ean,
    Cnid,
    Upc,
    Iban,
    Bin,
}
fn clean(s: &str, only_digits: bool) -> String {
    if only_digits {
        s.chars().filter(char::is_ascii_digit).collect()
    } else {
        s.chars()
            .filter(|&c| !space(c) && !"-./".contains(c))
            .collect::<String>()
            .to_uppercase()
    }
}
fn check(kind: Kind, body: &str) -> Result<String, CoreError> {
    if body.is_empty() || !body.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad("主体必须为十进制数字"));
    }
    let d: Vec<u32> = body.bytes().map(|b| (b - b'0') as u32).collect();
    let check = match kind {
        Kind::Luhn => {
            let sum: u32 = d
                .iter()
                .rev()
                .enumerate()
                .map(|(i, &v)| {
                    if i % 2 == 0 {
                        let v = v * 2;
                        if v > 9 {
                            v - 9
                        } else {
                            v
                        }
                    } else {
                        v
                    }
                })
                .sum();
            ((10 - sum % 10) % 10).to_string()
        }
        Kind::Isbn if d.len() == 9 => {
            let sum: u32 = d
                .iter()
                .enumerate()
                .map(|(i, &v)| v * (10 - i as u32))
                .sum();
            let n = (11 - sum % 11) % 11;
            if n == 10 {
                "X".into()
            } else {
                n.to_string()
            }
        }
        Kind::Ean | Kind::Isbn if d.len() == 12 => {
            let sum: u32 = d
                .iter()
                .enumerate()
                .map(|(i, &v)| v * if i % 2 == 0 { 1 } else { 3 })
                .sum();
            ((10 - sum % 10) % 10).to_string()
        }
        Kind::Upc if d.len() == 11 => {
            let sum: u32 = d
                .iter()
                .enumerate()
                .map(|(i, &v)| v * if i % 2 == 0 { 3 } else { 1 })
                .sum();
            ((10 - sum % 10) % 10).to_string()
        }
        Kind::Cnid if d.len() == 17 => {
            let weights = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
            let sum: u32 = d.iter().zip(weights).map(|(&n, w)| n * w).sum();
            (b"10X98765432"[(sum % 11) as usize] as char).to_string()
        }
        _ => return Err(bad("主体位数不符合算法要求")),
    };
    Ok(check)
}
fn mod97(s: &str) -> u32 {
    let mut rem = 0;
    for b in s.bytes() {
        if b.is_ascii_digit() {
            rem = (rem * 10 + (b - b'0') as u32) % 97;
        } else {
            rem = (rem * 100 + (b - b'A' + 10) as u32) % 97;
        }
    }
    rem
}
fn iban(s: &str, decode: bool) -> Result<PortMap, CoreError> {
    let s: String = s
        .chars()
        .filter(|&c| !space(c) && c != '-')
        .collect::<String>()
        .to_uppercase();
    if !s.is_ascii() || s.len() < if decode { 5 } else { 6 } {
        return Err(bad("IBAN 输入过短或非 ASCII"));
    }
    let cc = &s[..2];
    let start = if decode { 4 } else { 2 };
    let body = &s[start..];
    let len = TABLES["iban"][cc]
        .as_u64()
        .ok_or_else(|| bad("IBAN 国家代码不在固定版本长度表中"))? as usize;
    if body.len() != len - 4 || body.bytes().any(|b| !b.is_ascii_alphanumeric()) {
        return Err(bad("IBAN BBAN 长度或字符无效"));
    }
    if decode && !s[2..4].bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad("IBAN 校验位必须为两位数字"));
    }
    let expected = format!("{:02}", 98 - mod97(&format!("{body}{cc}00")));
    let full = format!("{cc}{expected}{body}");
    let mut o = one("check", PortValue::Text(expected.clone()));
    o.insert("full".into(), PortValue::Text(full.clone()));
    let text = if decode {
        let valid = mod97(&format!("{body}{cc}{}", &s[2..4])) == 1;
        o.insert("valid".into(), PortValue::Bool(valid));
        if valid {
            format!(
                "合法（{cc} IBAN mod-97 校验通过，长度 {} 符合该国规定）",
                s.len()
            )
        } else {
            format!("非法：校验位应为 {expected}（当前 {}）", &s[2..4])
        }
    } else {
        full
    };
    o.insert("text".into(), PortValue::Text(text));
    Ok(o)
}
fn bank(s: &str) -> Result<PortMap, CoreError> {
    let s = clean(s, true);
    if s.len() < 4 {
        return Err(bad("卡号至少 4 位"));
    }
    let found = TABLES["bank"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| s.starts_with(v["prefix"].as_str().unwrap()))
        .max_by_key(|v| v["prefix"].as_str().unwrap().len());
    let prefix = &s[..s.len().min(6)];
    let brand = found.and_then(|v| v["brand"].as_str()).unwrap_or("未知");
    let issuer = found.and_then(|v| v["issuer"].as_str()).unwrap_or("未知");
    let valid_len = (12..=19).contains(&s.len());
    let mut luhn = Value::Null;
    let mut lines = vec![
        format!(
            "卡号:        {s}{}",
            if s.len() >= 6 {
                ""
            } else {
                "（前 6 位不足，按已有前缀匹配）"
            }
        ),
        format!("BIN 前缀:    {prefix}"),
        format!("卡组织:      {brand}"),
        format!("发卡行:      {issuer}"),
        format!(
            "卡号长度:    {} 位{}",
            s.len(),
            if valid_len {
                "（合规）"
            } else {
                "（异常）"
            }
        ),
    ];
    if (13..=19).contains(&s.len()) {
        let valid = check(Kind::Luhn, &s[..s.len() - 1])? == s[s.len() - 1..];
        luhn = json!(valid);
        lines.push(format!(
            "Luhn 校验:   {}",
            if valid { "通过" } else { "不通过" }
        ));
    }
    Ok([("text".into(),PortValue::Text(lines.join("\n"))),("data".into(),PortValue::Json(json!({"number":s,"prefix":prefix,"matchedPrefix":found.map(|v|v["prefix"].clone()),"brand":brand,"issuer":issuer,"length":s.len(),"lengthInUpstreamRange":valid_len,"luhn":luhn,"referenceRevision":TABLES["revision"],"referenceOnly":true})))].into_iter().collect())
}
struct Check(Kind, bool);
impl Node for Check {
    fn run(&self, i: &PortMap, _p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let s = in_text(i, "text")?;
        if s.len() > 16384 {
            return Err(bad("校验输入最多 16384 字符"));
        }
        if matches!(self.0, Kind::Iban) {
            return iban(s, self.1);
        }
        if matches!(self.0, Kind::Bin) {
            return bank(s);
        }
        let digits_only = !(matches!(self.0, Kind::Isbn) || matches!(self.0, Kind::Cnid) && self.1);
        let s = clean(s, digits_only);
        if !s.is_ascii() || s.is_empty() {
            return Err(bad("校验输入为空或字符非法"));
        }
        let body = if self.1 {
            if s.len() < 2 {
                return Err(bad("完整号至少两位"));
            }
            let last = s.as_bytes()[s.len() - 1];
            if !(last.is_ascii_digit()
                || last == b'X' && matches!(self.0, Kind::Cnid | Kind::Isbn) && s.len() != 13)
            {
                return Err(bad("校验位字符非法"));
            }
            &s[..s.len() - 1]
        } else {
            &s
        };
        let expected = check(self.0, body)?;
        let mut out = one("check", PortValue::Text(expected.clone()));
        out.insert("full".into(), PortValue::Text(format!("{body}{expected}")));
        let text = if self.1 {
            let valid = s.ends_with(&expected);
            out.insert("valid".into(), PortValue::Bool(valid));
            if valid {
                match self.0 {
                    Kind::Luhn => "合法（Luhn 校验通过）",
                    Kind::Isbn if s.len() == 10 => "合法（ISBN-10 模 11 校验通过）",
                    Kind::Isbn => "合法（ISBN-13 模 10 校验通过）",
                    Kind::Ean => "合法（EAN-13 模 10 校验通过）",
                    Kind::Upc => "合法（UPC-A 模 10 校验通过）",
                    Kind::Cnid => "合法（GB 11643 校验通过）",
                    _ => unreachable!(),
                }
                .into()
            } else {
                format!(
                    "非法：{}校验位应为 {expected}",
                    if matches!(self.0, Kind::Luhn) {
                        "末位"
                    } else {
                        ""
                    }
                )
            }
        } else {
            expected
        };
        out.insert("text".into(), PortValue::Text(text));
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    for (kind, id, upstream, title, body, full) in [
        (
            Kind::Luhn,
            "luhn",
            "luhn",
            "Luhn 校验",
            "7992739871",
            "79927398713",
        ),
        (
            Kind::Isbn,
            "isbn",
            "isbn",
            "ISBN 校验",
            "030640615",
            "0306406152",
        ),
        (
            Kind::Ean,
            "ean13",
            "ean13",
            "EAN-13 校验",
            "400638133393",
            "4006381333931",
        ),
        (
            Kind::Cnid,
            "cnid_check",
            "cnidCheck",
            "身份证校验位",
            "00000000000000000",
            "000000000000000001",
        ),
        (
            Kind::Upc,
            "upc_a",
            "upc",
            "UPC-A 校验",
            "03600029145",
            "036000291452",
        ),
        (
            Kind::Iban,
            "iban",
            "iban",
            "IBAN 校验",
            "GBWEST12345698765432",
            "GB82WEST12345698765432",
        ),
    ] {
        for decode in [false, true] {
            let mut outputs = vec![
                t_out(),
                req("check", "期望校验位", PortType::Text),
                req("full", "含正确校验位的完整号", PortType::Text),
            ];
            if decode {
                outputs.push(req("valid", "校验通过", PortType::Bool));
            }
            let description=match kind{
                Kind::Iban=>"固定 ea2bc36 的 88 国长度表与 mod-97；计算输入国家代码+BBAN，验证输入完整 IBAN。仅检查长度和校验位，不验证账号真实性。",
                Kind::Isbn=>"自动按长度选择 ISBN-10/13；只移除空白、连字符、点和斜杠，末位允许 X；保留前导零。",
                Kind::Cnid=>"计算输入 17 位主体，验证输入 18 位完整字符串；只算 GB 11643 校验位，不验证地区、生日和身份。计算时按上游移除非数字。",
                _=>"计算输入不含校验位的主体，验证输入完整数字串；按上游移除非 ASCII 数字，保留前导零。提供布尔校验结果与期望校验位供下游。",
            };
            let d = document_text(
                desc(
                    &format!("{id}_{}", if decode { "validate" } else { "calculate" }),
                    "hash.checksum",
                    &format!("{title} · {}", if decode { "验证" } else { "计算" }),
                    AMBER,
                    vec![t_in()],
                    outputs,
                    vec![],
                ),
                upstream,
                description,
                if decode { full } else { body },
                json!({}),
            );
            reg.register(d, Arc::new(move || Arc::new(Check(kind, decode))));
        }
    }
    let mut d=document_text(desc("bank_bin_identify","numeric.address","银行卡 BIN 前缀识别",CYAN,vec![t_in()],vec![t_out(),req("data","前缀识别明细",PortType::Json)],vec![]),"bankBin","按固定 ea2bc36 的简化 BIN 表最长前缀匹配，并检查长度与 Luhn。此表为迁移快照，不能用于确认实时发卡行或卡号真实性。","621700",json!({}));
    d.aliases.extend(["BIN".into(), "银行卡前缀".into()]);
    reg.register(d, Arc::new(|| Arc::new(Check(Kind::Bin, false))));
}
