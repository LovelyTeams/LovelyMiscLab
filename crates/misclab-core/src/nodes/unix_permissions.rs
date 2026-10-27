//! Offline UNIX mode inspection; never changes filesystem permissions.
use super::{codec_support::document_text, prelude::*, transport_codecs::err};
use serde_json::json;

struct Permissions;
impl Node for Permissions {
    fn run(
        &self,
        inputs: &PortMap,
        _: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        ctx.cancel.check()?;
        let text = in_text(inputs, "text")?.trim();
        if text.len() > 4096 {
            return Err(err("权限输入过长"));
        }
        let s: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        let octal = (s.len() == 3 || s.len() == 4) && s.bytes().all(|b| (b'0'..=b'7').contains(&b));
        let mut digits = [0u8; 4];
        let mut kind = '-';
        if octal {
            for (i, b) in s.bytes().enumerate() {
                digits[4 - s.len() + i] = b - b'0';
            }
        } else {
            let pattern = regex::Regex::new(
                r"^([dlpcbsD-]?)([r-][w-][xsS-])([r-][w-][xsS-])([r-][w-][xtT-])$",
            )
            .unwrap();
            let caps = pattern.captures(&s).ok_or_else(|| {
                err("权限需要 3–4 位八进制或 9–10 位符号形，例如 4755、drwxr-xr-t")
            })?;
            kind = caps[1].chars().next().unwrap_or('-');
            for i in 0..3 {
                let b = caps[i + 2].as_bytes();
                digits[i + 1] = if b[0] == b'r' { 4 } else { 0 }
                    | if b[1] == b'w' { 2 } else { 0 }
                    | if b"xst".contains(&b[2]) { 1 } else { 0 };
                if b"sStT".contains(&b[2]) {
                    digits[0] |= 1 << (2 - i);
                }
            }
        }
        let [sp, u, g, o] = digits;
        let mut sym = String::new();
        let mut chmod = Vec::new();
        for (i, d) in [u, g, o].into_iter().enumerate() {
            let mut simple = String::new();
            for (mask, c) in [(4, 'r'), (2, 'w')] {
                sym.push(if d & mask != 0 { c } else { '-' });
                if d & mask != 0 {
                    simple.push(c);
                }
            }
            let x = d & 1 != 0;
            if x {
                simple.push('x');
            }
            let special = sp & (1 << (2 - i)) != 0;
            sym.push(match (special, x, i == 2) {
                (true, true, false) => 's',
                (true, false, false) => 'S',
                (true, true, true) => 't',
                (true, false, true) => 'T',
                (_, true, _) => 'x',
                _ => '-',
            });
            chmod.push(format!("{}={simple}", ['u', 'g', 'o'][i]));
        }
        for (bit, part) in [(4, "u+s"), (2, "g+s"), (1, "o+t")] {
            if sp & bit != 0 {
                chmod.push(part.into());
            }
        }
        let kind_name = match kind {
            'd' => "目录（d）",
            'l' => "符号链接（l）",
            'p' => "命名管道 FIFO（p）",
            'c' => "字符设备（c）",
            'b' => "块设备（b）",
            's' => "套接字（s）",
            'D' => "门 door·Solaris（D）",
            _ => "未知（八进制/9 位符号形不含类型位，按普通文件 - 展示）",
        };
        let oct3 = format!("{u}{g}{o}");
        let oct4 = format!("{sp}{oct3}");
        let binary = format!(
            "{u:03b} {g:03b} {o:03b}{}",
            if sp > 0 {
                format!("（特殊位 {sp:03b}）")
            } else {
                String::new()
            }
        );
        let symbolic_chmod = chmod.join(",");
        let mut lines = vec![
            format!(
                "输入形态：{} {text}",
                if octal { "八进制" } else { "符号形" }
            ),
            format!("符号形（9 位）：{sym}"),
            format!("带类型位（10 位）：{kind}{sym}"),
            format!("文件类型：{kind_name}"),
            format!("八进制（3 位）：{oct3}"),
            format!("八进制（4 位，含特殊位）：{oct4}"),
            format!("二进制位：{binary}"),
            format!("chmod 命令：chmod {oct4} 文件"),
            format!("符号 chmod：chmod {symbolic_chmod} 文件"),
        ];
        let mut specials = Vec::new();
        for (bit,description) in [
            (4,"setuid（首位 4）：以文件属主身份执行——符号形属主执行位显示 s（有执行）/ S（无执行）"),
            (2,"setgid（首位 2）：以文件属组身份执行；用在目录上则新文件继承目录属组——符号形属组执行位显示 s/S"),
            (1,"sticky（首位 1）：仅属主（及 root）可删改目录内文件，典型如 /tmp 的 1777——符号形其他执行位显示 t/T")
        ] { if sp&bit!=0 {specials.push(description);} }
        lines.push(if sp == 0 {
            "特殊位：无（setuid / setgid / sticky 均未设置）".into()
        } else {
            format!(
                "特殊位（八进制首位 {sp}）：\n  - {}",
                specials.join("\n  - ")
            )
        });
        for (label, d) in [
            ("属主 owner（u）", u),
            ("属组 group（g）", g),
            ("其他 other（o）", o),
        ] {
            let names: Vec<_> = [(4, "读 r"), (2, "写 w"), (1, "执行 x")]
                .into_iter()
                .filter(|(mask, _)| d & mask != 0)
                .map(|(_, s)| s)
                .collect();
            lines.push(format!(
                "{label}：{}",
                if names.is_empty() {
                    "无任何权限（---）".into()
                } else {
                    names.join(" + ")
                }
            ));
        }
        let mut out = out_text(lines.join("\n"));
        out.insert("data".into(),PortValue::Json(json!({"octal":oct4,"symbolic":sym,"type":kind.to_string(),"owner":u,"group":g,"other":o,"special":sp,"chmod":symbolic_chmod})));
        Ok(out)
    }
}
pub fn register(reg: &mut NodeRegistry) {
    let mut d = document_text(desc("unix_permissions","numeric.bits","UNIX 文件权限",CYAN,vec![t_in()],vec![t_out(),req("data","权限明细",PortType::Json)],vec![]),"unixPerms","解析八进制或符号形 UNIX 权限，包含文件类型、setuid/setgid/sticky、二进制与 chmod 文本。仅生成说明，不修改文件权限。","4755",json!({}));
    d.aliases
        .extend(["chmod", "POSIX permissions", "rwx"].map(String::from));
    reg.register(d, Arc::new(|| Arc::new(Permissions)));
}
