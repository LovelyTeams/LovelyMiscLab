//! 全角/半角转换：全角 ASCII（Ａ０！等，U+FF01..FF5E）↔ 半角（U+21..7E），全角空格
//! U+3000 ↔ 半角空格。CJK 场景常把 flag 塞成全角。
use super::codec_support::document_text;
use super::prelude::*;
use serde_json::json;

fn full_to_half(c: char) -> char {
    match c as u32 {
        0x3000 => ' ',
        cp @ 0xFF01..=0xFF5E => char::from_u32(cp - 0xFEE0).unwrap_or(c),
        _ => c,
    }
}

fn half_to_full(c: char) -> char {
    match c as u32 {
        0x20 => '\u{3000}',
        cp @ 0x21..=0x7E => char::from_u32(cp + 0xFEE0).unwrap_or(c),
        _ => c,
    }
}

struct N;
impl Node for N {
    fn run(
        &self,
        i: &PortMap,
        p: &serde_json::Value,
        c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        let text = in_text(i, "text")?;
        if text.len() > 1_048_576 {
            return Err(CoreError::Parse("全半角输入最多1MiB".into()));
        }
        c.cancel.check()?;
        let f = if pstr(p, "direction", "全角→半角") == "半角→全角" {
            half_to_full
        } else {
            full_to_half
        };
        let mut out = String::new();
        for (index, ch) in text.char_indices() {
            if index % 1024 < ch.len_utf8() {
                c.progress(index as f32 / text.len().max(1) as f32);
                c.cancel.check()?;
            }
            out.push(f(ch));
        }
        c.progress(1.0);
        c.cancel.check()?;
        Ok(out_text(out))
    }
}
struct Direction(bool);
impl Node for Direction {
    fn run(
        &self,
        i: &PortMap,
        _p: &serde_json::Value,
        c: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        N.run(
            i,
            &json!({"direction":if self.0{"全角→半角"}else{"半角→全角"}}),
            c,
        )
    }
}

pub fn register(reg: &mut NodeRegistry) {
    for decode in [false, true] {
        let d=document_text(desc(if decode{"fullwidth_decode"}else{"fullwidth_encode"},"encoding.charset",if decode{"全角转半角"}else{"半角转全角"},BLUE,vec![t_in()],vec![t_out()],vec![]),"fullwidth","ASCII可打印字符与全角形式互转，空格对应U+3000；其他Unicode字符原样保留，原有全角/半角节点继续兼容。",if decode{"ｆｌａｇ　中文🙂"}else{"flag 中文🙂"},json!({}));
        reg.register(d, Arc::new(move || Arc::new(Direction(decode))));
    }
    reg.register(
        desc(
            "fullwidth_halfwidth",
            CHARSET,
            "全角/半角",
            BLUE,
            vec![t_in()],
            vec![t_out()],
            vec![ParamSpec::select(
                "direction",
                "方向",
                &["全角→半角", "半角→全角"],
                "全角→半角",
            )],
        ),
        Arc::new(|| Arc::new(N)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::CancellationToken;
    use crate::graph::executor::GraphExecutor;
    use crate::nodes::default_registry;
    use crate::progress::NullSink;

    fn run(text: &str, dir: &str) -> String {
        let mut i = PortMap::new();
        i.insert("text".into(), PortValue::Text(text.into()));
        let out = GraphExecutor::run_node(
            &default_registry(),
            "fullwidth_halfwidth",
            &i,
            &serde_json::json!({ "direction": dir }),
            &NullSink,
            &CancellationToken::new(),
        )
        .unwrap();
        match out.get("text") {
            Some(PortValue::Text(s)) => s.clone(),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn converts_both_ways() {
        let full = "ｆｌａｇ｛１２３｝";
        assert_eq!(run(full, "全角→半角"), "flag{123}");
        assert_eq!(run("flag{123}", "半角→全角"), full);
    }
}
