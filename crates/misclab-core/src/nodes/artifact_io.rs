use super::prelude::*;
use crate::artifacts::{repository, MAX_ARTIFACT_BYTES};

struct Store;
impl Node for Store {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        ctx.check_cancel()?;
        let data = in_bytes(inputs, "bytes")?;
        let meta = repository().put(
            pstr(p, "name", "output.bin"),
            pstr(p, "mime", "application/octet-stream"),
            &data,
        )?;
        let mut out = one("artifact", PortValue::Artifact(meta.id));
        out.insert(
            "metadata".into(),
            PortValue::Json(
                serde_json::to_value(meta).map_err(|e| CoreError::Other(e.to_string()))?,
            ),
        );
        Ok(out)
    }
}
struct Read;
impl Node for Read {
    fn run(
        &self,
        inputs: &PortMap,
        p: &serde_json::Value,
        ctx: &mut NodeCtx,
    ) -> Result<PortMap, CoreError> {
        ctx.check_cancel()?;
        let id = match inputs.get("artifact") {
            Some(PortValue::Artifact(id)) => *id,
            _ => return Err(CoreError::Type("需要 Artifact 文件产物引用".into())),
        };
        let max = pnum(p, "maxBytes", 16_777_216.0);
        if !max.is_finite()
            || max.fract() != 0.0
            || !(0.0..=MAX_ARTIFACT_BYTES as f64).contains(&max)
        {
            return Err(CoreError::Parse(
                "最大字节数需要是 0 到 128 MiB 之间的整数".into(),
            ));
        }
        let data = repository().read_all(id, max as usize)?;
        ctx.check_cancel()?;
        Ok(one("bytes", PortValue::Bytes(data.into())))
    }
}
pub fn register(reg: &mut NodeRegistry) {
    let mut store = desc(
        "artifact_store",
        IO,
        "保存为文件产物",
        SLATE,
        vec![req("bytes", "文件字节", PortType::Bytes)],
        vec![
            req("artifact", "文件产物", PortType::Artifact),
            req("metadata", "元数据", PortType::Json),
        ],
        vec![
            ParamSpec::text("name", "文件名", "output.bin", false),
            ParamSpec::text("mime", "MIME 类型", "application/octet-stream", false),
        ],
    );
    store.description =
        "把字节保存到持久化产物库，输出 UUID 引用及 SHA-256；后续通过读取产物节点恢复字节。".into();
    store.params[0].description = "导出时建议使用的文件名；路径部分会移除。".into();
    store.params[1].description =
        "用于选择图片、音频或视频预览；未知二进制使用 application/octet-stream。".into();
    store.examples.push(crate::node::descriptor::NodeExample {
        label: "保存二进制样例".into(),
        inputs: one("bytes", PortValue::Bytes(vec![0, 255, 65].into())),
        params: serde_json::json!({ "name": "sample.bin" }),
    });
    reg.register(store, Arc::new(|| Arc::new(Store)));
    let mut read = desc(
        "artifact_read",
        IO,
        "读取文件产物",
        SLATE,
        vec![req("artifact", "文件产物", PortType::Artifact)],
        vec![req("bytes", "文件字节", PortType::Bytes)],
        vec![ParamSpec::number(
            "maxBytes",
            "最大字节数",
            0.0,
            MAX_ARTIFACT_BYTES as f64,
            1.0,
            16_777_216.0,
        )],
    );
    read.description = "显式把产物 UUID 转为原始字节，超过大小限制会报错。".into();
    read.inputs[0].description =
        "连接保存产物 / Data URI 解析的 artifact 输出，或在独立调用中上传文件。".into();
    read.params[0].description =
        "读取前先检查产物大小；默认 16 MiB，上限 128 MiB，0 只允许空文件。".into();
    reg.register(read, Arc::new(|| Arc::new(Read)));
}
