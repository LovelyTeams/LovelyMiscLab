//! A local fixed-width expression parser. No eval or external interpreter.
//! Operator semantics follow EBCTFCodeBox ea2bc36/progCalc.js.
use super::{
    codec_support::{document_text, help},
    numeric_support,
    prelude::*,
    prime_tools::modpow,
    transport_codecs::err,
};
use num_bigint::{BigInt, BigUint};
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};
use serde_json::{json, Value};

#[derive(Clone)]
enum Kind {
    Number(BigUint),
    Ident(String),
    Op(&'static str),
}
#[derive(Clone)]
struct Token {
    kind: Kind,
    pos: usize,
}
fn lex(src: &str, ctx: &NodeCtx) -> Result<Vec<Token>, CoreError> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < src.len() {
        if i.is_multiple_of(16) {
            ctx.progress(0.04 * i as f32 / src.len() as f32);
            ctx.cancel.check()?;
        }
        let c = src[i..].chars().next().unwrap();
        if numeric_support::space(c) {
            i += c.len_utf8();
            continue;
        }
        let start = i;
        let kind = if c.is_ascii_digit() {
            let mut radix = 10;
            if src.as_bytes()[i] == b'0' && i + 1 < src.len() {
                radix = match src.as_bytes()[i + 1].to_ascii_lowercase() {
                    b'x' => 16,
                    b'b' => 2,
                    b'o' => 8,
                    _ => 10,
                };
                if radix != 10 {
                    i += 2;
                }
            }
            let digits = i;
            while i < src.len() && (src.as_bytes()[i] as char).is_digit(radix) {
                i += 1;
            }
            if i == digits {
                return Err(err(format!("位置 {}：进制前缀后缺少数字", start + 1)));
            }
            Kind::Number(
                BigUint::parse_bytes(&src.as_bytes()[digits..i], radix)
                    .ok_or_else(|| err("数字字面量无效"))?,
            )
        } else if c.is_ascii_alphabetic() || c == '_' {
            i += 1;
            while i < src.len()
                && (src.as_bytes()[i].is_ascii_alphanumeric() || src.as_bytes()[i] == b'_')
            {
                i += 1;
            }
            Kind::Ident(src[start..i].into())
        } else {
            let op = [
                ">>>", "<<", ">>", "**", "&", "|", "^", "~", "+", "-", "*", "/", "%", "(", ")", ",",
            ]
            .into_iter()
            .find(|op| src[i..].starts_with(op));
            let op = op.ok_or_else(|| {
                err(format!(
                    "位置 {}：无法识别字符 {c}",
                    src[..i].encode_utf16().count() + 1
                ))
            })?;
            i += op.len();
            Kind::Op(op)
        };
        out.push(Token {
            kind,
            pos: src[..start].encode_utf16().count() + 1,
        });
    }
    Ok(out)
}
struct Parser<'a, 'b> {
    tokens: Vec<Token>,
    at: usize,
    groups: usize,
    width: usize,
    signed: bool,
    modulus: BigUint,
    mask: BigUint,
    ctx: &'a NodeCtx<'b>,
}
impl Parser<'_, '_> {
    fn signed_value(&self, n: &BigUint) -> BigInt {
        let value = BigInt::from(n.clone());
        if n.bit(self.width as u64 - 1) {
            value - BigInt::from(self.modulus.clone())
        } else {
            value
        }
    }
    fn wrap(&self, n: BigInt) -> BigUint {
        n.mod_floor(&BigInt::from(self.modulus.clone()))
            .into_parts()
            .1
    }
    fn count(&self, n: &BigUint) -> BigInt {
        if self.signed {
            self.signed_value(n)
        } else {
            n.clone().into()
        }
    }
    fn peek_op(&self) -> Option<&'static str> {
        match self.tokens.get(self.at).map(|t| &t.kind) {
            Some(Kind::Op(op)) => Some(op),
            _ => None,
        }
    }
    fn take(&mut self) -> Result<Token, CoreError> {
        let token = self
            .tokens
            .get(self.at)
            .cloned()
            .ok_or_else(|| err("表达式意外结束，缺少操作数或右括号"))?;
        self.at += 1;
        self.ctx
            .progress(0.05 + 0.85 * self.at as f32 / self.tokens.len() as f32);
        self.ctx.cancel.check()?;
        Ok(token)
    }
    fn expect(&mut self, op: &str) -> Result<(), CoreError> {
        let t = self.take()?;
        if matches!(t.kind,Kind::Op(x) if x==op) {
            Ok(())
        } else {
            Err(err(format!("位置 {}：期望 {op}", t.pos)))
        }
    }
    fn depth(&self, depth: usize) -> Result<(), CoreError> {
        if depth > 128 {
            Err(err("运算符递归深度超过 128，请拆分表达式"))
        } else {
            self.ctx.cancel.check()
        }
    }
    fn enter_group(&mut self) -> Result<(), CoreError> {
        self.groups += 1;
        if self.groups > 32 {
            Err(err("括号或函数嵌套超过 32 层"))
        } else {
            Ok(())
        }
    }
    fn primary(&mut self, depth: usize) -> Result<BigUint, CoreError> {
        let t = self.take()?;
        match t.kind {
            Kind::Number(n) => Ok(n & &self.mask),
            Kind::Ident(name) => {
                if !["rotl", "rotr"].contains(&name.as_str()) {
                    return Err(err(format!(
                        "位置 {}：未知函数 {name}，仅支持 rotl/rotr",
                        t.pos
                    )));
                }
                self.expect("(")?;
                self.enter_group()?;
                let x = self.expr(0, depth + 1)?;
                self.expect(",")?;
                let count = self.expr(0, depth + 1)?;
                self.expect(")")?;
                self.groups -= 1;
                let n = self
                    .count(&count)
                    .mod_floor(&BigInt::from(self.width))
                    .to_usize()
                    .unwrap();
                Ok(if name == "rotl" {
                    ((&x << n) | (&x >> (self.width - n))) & &self.mask
                } else {
                    ((&x >> n) | (&x << (self.width - n))) & &self.mask
                })
            }
            Kind::Op("(") => {
                self.enter_group()?;
                let n = self.expr(0, depth + 1)?;
                self.expect(")")?;
                self.groups -= 1;
                Ok(n)
            }
            _ => Err(err(format!("位置 {}：缺少操作数", t.pos))),
        }
    }
    fn unary(&mut self, depth: usize) -> Result<BigUint, CoreError> {
        self.depth(depth)?;
        if let Some(op @ ("~" | "-" | "+")) = self.peek_op() {
            self.take()?;
            let n = self.unary(depth + 1)?;
            Ok(match op {
                "~" => &self.mask ^ n,
                "-" if !n.is_zero() => &self.modulus - n,
                _ => n,
            })
        } else {
            self.primary(depth)
        }
    }
    fn expr(&mut self, min: u8, depth: usize) -> Result<BigUint, CoreError> {
        self.depth(depth)?;
        let mut lhs = self.unary(depth)?;
        while let Some(op) = self.peek_op() {
            let prec = match op {
                "|" => 1,
                "^" => 2,
                "&" => 3,
                "<<" | ">>" | ">>>" => 4,
                "+" | "-" => 5,
                "*" | "/" | "%" => 6,
                "**" => 7,
                _ => break,
            };
            if prec < min {
                break;
            }
            self.take()?;
            let rhs = self.expr(if op == "**" { prec } else { prec + 1 }, depth + 1)?;
            lhs = self.binary(op, lhs, rhs)?;
        }
        Ok(lhs)
    }
    fn binary(&self, op: &str, a: BigUint, b: BigUint) -> Result<BigUint, CoreError> {
        self.ctx.cancel.check()?;
        Ok(match op {
            "|" => a | b,
            "^" => a ^ b,
            "&" => a & b,
            "+" => (a + b) & &self.mask,
            "-" => {
                if a >= b {
                    a - b
                } else {
                    &self.modulus + a - b
                }
            }
            "*" => (a * b) & &self.mask,
            "/" | "%" => {
                if b.is_zero() {
                    return Err(err("除数/模数不能为零（按字宽回绕后）"));
                }
                if self.signed {
                    let (a, b) = (self.signed_value(&a), self.signed_value(&b));
                    self.wrap(if op == "/" { a / b } else { a % b })
                } else if op == "/" {
                    a / b
                } else {
                    a % b
                }
            }
            "<<" | ">>" | ">>>" => {
                let n = self.count(&b);
                if n.is_negative() {
                    return Err(err("移位数为负；signed=false 时按无符号位模式解读计数"));
                }
                let n = (n % self.width).to_usize().unwrap();
                match op {
                    "<<" => (a << n) & &self.mask,
                    ">>" => self.wrap(self.signed_value(&a) >> n),
                    _ => a >> n,
                }
            }
            "**" => {
                let exp = self.count(&b);
                if exp.is_negative() {
                    return Err(err("整数计算器不支持负指数"));
                }
                modpow(
                    a,
                    exp.magnitude(),
                    &self.modulus,
                    self.ctx,
                    0.05 + 0.85 * self.at as f32 / self.tokens.len() as f32,
                    0.0,
                )?
            }
            _ => unreachable!(),
        })
    }
}

struct ProgrammerCalc;
impl Node for ProgrammerCalc {
    fn run(&self, inputs: &PortMap, p: &Value, ctx: &mut NodeCtx) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let raw = in_text(inputs, "text")?;
        if raw.len() > 16384 {
            return Err(err("表达式原始输入最多 16384 字节"));
        }
        let src = raw.trim_matches(numeric_support::space);
        if src.is_empty() || src.encode_utf16().count() > 1024 {
            return Err(err("表达式需为 1–1024 字符"));
        }
        let width = numeric_support::number(p, "width", 32, 1, 512)? as usize;
        let signed = match p.get("signed") {
            None => true,
            Some(v) => v.as_bool().ok_or_else(|| err("signed 必须为布尔值"))?,
        };
        let modulus = BigUint::one() << width;
        let mut parser = Parser {
            tokens: lex(src, ctx)?,
            at: 0,
            groups: 0,
            width,
            signed,
            mask: &modulus - 1u8,
            modulus,
            ctx,
        };
        let value = parser.expr(0, 0)?;
        if let Some(t) = parser.tokens.get(parser.at) {
            return Err(err(format!("位置 {}：表达式结束后仍有多余内容", t.pos)));
        }
        let sv = parser.signed_value(&value);
        let selected = if signed {
            sv.to_string()
        } else {
            value.to_string()
        };
        let hex = format!(
            "0x{:0>length$}",
            value.to_str_radix(16).to_uppercase(),
            length = width.div_ceil(4)
        );
        let binary = format!("{:0>width$}", value.to_str_radix(2));
        let grouped = binary
            .as_bytes()
            .chunks(4)
            .map(|s| std::str::from_utf8(s).unwrap())
            .collect::<Vec<_>>()
            .join(" ");
        let octal = format!("0o{}", value.to_str_radix(8));
        let popcount = value
            .to_u64_digits()
            .iter()
            .map(|n| n.count_ones())
            .sum::<u32>();
        let leading = width - value.bits() as usize;
        let trailing = value.trailing_zeros().unwrap_or(width as u64);
        let complement = if sv.is_negative() {
            format!("{sv} 的 {width} 位补码 = {hex}（无符号值 {value}）")
        } else {
            format!("非负数，补码与原码相同（{hex}）")
        };
        let report = format!("结果：{selected}（按 {width} 位 {}解读）\n  十进制   : {sv}（signed） / {value}（unsigned）\n  十六进制 : {hex}\n  八进制   : {octal}\n  二进制   : {grouped}\n  补码     : {complement}\n  置位数   : {popcount}\n  前导零   : {leading}（按 {width} 位字宽）\n  尾随零   : {trailing}",if signed {"有符号"} else {"无符号"});
        let mut bytes = value.to_bytes_be();
        let padding = width.div_ceil(8) - bytes.len();
        if padding != 0 {
            let mut padded = vec![0; padding];
            padded.append(&mut bytes);
            bytes = padded;
        }
        let mut out = out_text(report);
        out.insert("value".into(), PortValue::Text(selected.clone()));
        out.insert("signed".into(), PortValue::Text(sv.to_string()));
        out.insert("unsigned".into(), PortValue::Text(value.to_string()));
        out.insert("bytes".into(), PortValue::Bytes(bytes.into()));
        out.insert("json".into(),PortValue::Json(json!({"expression":src,"width":width,"signedMode":signed,"value":selected,"signed":sv.to_string(),"unsigned":value.to_string(),"hex":hex,"octal":octal,"binary":binary,"popcount":popcount,"leadingZeros":leading,"trailingZeros":trailing,"byteOrder":"big-endian"})));
        ctx.progress(1.0);
        ctx.cancel.check()?;
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    let mut d = document_text(desc("programmer_calc","numeric.bits","程序员计算器",CYAN,vec![t_in()],
        vec![t_out(),req("value","当前模式十进制",PortType::Text),req("signed","有符号十进制",PortType::Text),req("unsigned","无符号十进制",PortType::Text),req("bytes","大端字节（左侧补零）",PortType::Bytes),req("json","进制视图与位统计",PortType::Json)],
        vec![help(ParamSpec::number("width","字宽",1.0,512.0,1.0,32.0),"1–512 位（常用 8/16/32/64）；字面量和每步结果立即按 2^width 回绕。字节输出按 ceil(width/8) 大端排列，高位不足一字节时补零。"),
        help(ParamSpec::toggle("signed","有符号解读",true),"影响结果显示、除法/余数和移位数/指数的符号解读。>> 始终算术右移，>>> 始终逻辑右移；移位数按字宽取模，rotl/rotr 支持负计数。")]),
        "progCalc","本地表达式求值：& | ^ ~ << >> >>> + - * / % **、括号和 rotl/rotr；支持十进制、0x/0b/0o。优先级从低到高为 |、^、&、移位、加减、乘除余、幂、一元；幂右结合。最多 1024 字符，32 层括号/函数和 128 层运算递归，不执行任意代码。","rotl(0x80000000, 1) | 0xA0",json!({}));
    d.family = "fixed width integer bitwise arithmetic".into();
    d.aliases.extend(
        [
            "progCalc",
            "programmer calculator",
            "rotl",
            "rotr",
            "位运算",
            "补码计算",
            "循环移位",
        ]
        .map(String::from),
    );
    d.presentation.inline_params = vec!["width".into(), "signed".into()];
    reg.register(d, Arc::new(|| Arc::new(ProgrammerCalc)));
}
