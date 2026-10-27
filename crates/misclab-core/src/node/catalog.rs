//! Shared, stable taxonomy and presentation metadata. Registration enriches old
//! descriptors without changing any executable id, port or parameter.
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use super::descriptor::{NodeDescriptor, ParamWidget};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub order: usize,
    pub icon: String,
    pub color: String,
}

pub fn categories() -> &'static [Category] {
    static CATALOG: OnceLock<Vec<Category>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let groups = [
            ("io", "输入输出", "FileInput", "#64748b", "sources:输入与输出"),
            ("encoding", "编码解码", "Binary", "#22c55e", "base:Base 编码,transport:传输编码,charset:字符集,fancy:花式编码,chinese:中文编码"),
            ("data", "文本与数据", "Type", "#14b8a6", "text:文本处理,serialization:序列化,structure:结构解析"),
            ("numeric", "进制与数值", "Calculator", "#06b6d4", "radix:进制,bits:位与字节,math:数论,time:时间日期,address:地址与单位"),
            ("classical", "古典密码", "KeyRound", "#a855f7", "substitution:替换密码,transposition:置换密码,polygraphic:多表与分组,rotor:转轮密码"),
            ("crypto", "现代密码", "LockKeyhole", "#6366f1", "block:分组密码,stream:流密码,asymmetric:非对称密码,auth:签名与认证"),
            ("hash", "哈希与校验", "Hash", "#f59e0b", "digest:摘要,checksum:校验"),
            ("analysis", "分析与破解", "ScanSearch", "#d946ef", "statistics:统计分析,detect:自动识别,crypto:密码分析,password:口令破解"),
            ("archive", "压缩与归档", "Archive", "#f59e0b", "compression:压缩,container:归档"),
            ("forensics", "文件与取证", "FileSearch", "#3b82f6", "structure:文件结构,document:文档与 APK,binary:可执行文件,network:网络流量"),
            ("stego", "隐写与水印", "EyeOff", "#a855f7", "text:文本载体,image:图像载体,audio:音频载体,file:文件载体"),
            ("image", "图像与二维码", "Image", "#f43f5e", "process:图像处理,qr:二维码"),
            ("media", "音频与视频", "AudioLines", "#d946ef", "audio:音频分析,convert:音视频转换"),
            ("language", "语言与执行", "Code", "#22c55e", "esolang:深奥语言,code:代码编码解析"),
            ("visual", "可视化分析", "ChartColumn", "#06b6d4", "charts:数据图表"),
            ("control", "控制与逻辑", "Workflow", "#64748b", "flow:流程控制"),
            ("ai", "AI", "Sparkles", "#a855f7", "models:模型调用"),
            ("custom", "自定义", "Boxes", "#8b5cf6", "modules:用户模块"),
        ];
        let mut out = Vec::new();
        for (id, name, icon, color, children) in groups {
            out.push(Category { id: id.into(), name: name.into(), parent_id: None, order: out.len(), icon: icon.into(), color: color.into() });
            for child in children.split(',') {
                let (key, label) = child.split_once(':').unwrap();
                out.push(Category { id: format!("{id}.{key}"), name: label.into(), parent_id: Some(id.into()), order: out.len(), icon: icon.into(), color: color.into() });
            }
        }
        out
    })
}

pub fn category_order(id: &str) -> usize {
    categories()
        .iter()
        .find(|c| c.id == id)
        .map_or(usize::MAX, |c| c.order)
}

fn classify(d: &NodeDescriptor) -> &str {
    if categories()
        .iter()
        .any(|c| c.id == d.category && c.parent_id.is_some())
    {
        return &d.category;
    }
    let id = d.id.as_str();
    if id.starts_with("mod_") || id.starts_with("script_") {
        return "custom.modules";
    }
    let exact = match id {
        "hmac" | "jwt_decode" => "crypto.auth",
        "hash_crack" | "password_crack" | "jwt_crack" | "zip_crc_crack" | "bkcrack" => {
            "analysis.password"
        }
        "base64_stego" => "stego.text",
        "brainfuck" => "language.esolang",
        "roman_numeral" | "number_theory" | "factorize" | "crt" => "numeric.math",
        "rotate_bytes" | "endian_swap" | "bitwise" => "numeric.bits",
        "ip_convert" => "numeric.address",
        "to_timestamp" | "from_timestamp" => "numeric.time",
        "json_format" => "data.serialization",
        "extract" | "defang" => "data.text",
        "entropy" | "char_frequency" | "text_score" => "analysis.statistics",
        "magic_decode" | "loop_decode" => "analysis.detect",
        "xor_bruteforce"
        | "repeating_xor_crack"
        | "affine_bruteforce"
        | "caesar_brute"
        | "columnar_bruteforce"
        | "rail_fence_bruteforce"
        | "vigenere_analyze" => "analysis.crypto",
        "aes" | "des" | "blowfish" | "cryptojs_aes" => "crypto.block",
        "chacha20" | "salsa20" | "rc4" | "rabbit" | "xor" => "crypto.stream",
        "rsa_params" | "rsa_decrypt" | "pgp_decrypt" => "crypto.asymmetric",
        "pgp_enarmor" | "pgp_dearmor" => "encoding.transport",
        "vigenere" | "adfgvx" | "polybius" => "classical.polygraphic",
        "enigma" => "classical.rotor",
        "phone_keypad" | "tap_code" | "keyboard_shift" | "substitution" | "affine" | "caesar"
        | "atbash" | "rot5" | "rot13" | "rot18" | "rot47" => "classical.substitution",
        "compress" => "archive.compression",
        "http_request" => "forensics.network",
        "exif_extract" | "file_carve" | "detect_file_type" | "png_chunks" | "gif_blocks"
        | "jpeg_markers" => "forensics.structure",
        "apk_manifest" | "ooxml_metadata" | "ooxml_embedded" | "pdf_objects" | "pdf_streams" => {
            "forensics.document"
        }
        "deepsound_extract" | "wav_lsb" => "stego.audio",
        "fullwidth_halfwidth" | "encode_text" | "decode_text" => "encoding.charset",
        "from_hexdump" | "to_hexdump" => "numeric.bits",
        _ => "",
    };
    if !exact.is_empty() {
        return exact;
    }
    for (prefixes, category) in [
        ("base,radix64,crockford32,modhex,hex_,z85", "encoding.base"),
        (
            "url_,html_entity,unicode_,quoted_printable,uuencode,xxencode,citrix_ctx1",
            "encoding.transport",
        ),
        ("rail_fence", "classical.transposition"),
        ("bifid,playfair", "classical.polygraphic"),
        ("qr_", "image.qr"),
        (
            "whitespace,zero_width,stegcloak,text_blind_watermark",
            "stego.text",
        ),
        ("blind_watermark,guofei_bwm", "stego.image"),
    ] {
        if prefixes.split(',').any(|p| id.starts_with(p)) {
            return category;
        }
    }
    match d.category.as_str() {
        "输入输出" => "io.sources",
        "编码/加密" => "encoding.fancy",
        "文本处理" => "data.text",
        "控制/逻辑" => "control.flow",
        "压缩包" => "archive.container",
        "隐写术" => "stego.image",
        "哈希/摘要" => "hash.digest",
        "进制转换" => "numeric.radix",
        "加密解密" => "crypto.block",
        "字符编码" => "encoding.charset",
        "工具/分析" => "analysis.detect",
        "图像处理" => "image.process",
        "音频处理" => "media.audio",
        "AI" => "ai.models",
        "二进制分析" => "forensics.binary",
        "可视化分析" => "visual.charts",
        _ => "custom.modules",
    }
}

pub fn enrich(d: &mut NodeDescriptor) {
    if d.category_id.is_empty() {
        let sub = classify(d).to_owned();
        let leaf = categories().iter().find(|c| c.id == sub).unwrap();
        let parent = categories()
            .iter()
            .find(|c| Some(&c.id) == leaf.parent_id.as_ref())
            .unwrap();
        d.legacy_categories.push(d.category.clone());
        d.category_id = parent.id.clone();
        d.subcategory_id = sub;
        d.category = parent.name.clone();
        if d.category_id != "custom" {
            d.color = parent.color.clone();
        }
    }
    let stem = ["_encode", "_decode", "_encrypt", "_decrypt"]
        .iter()
        .find_map(|suffix| d.id.strip_suffix(suffix))
        .unwrap_or(&d.id)
        .to_string();
    if d.family.is_empty() {
        d.family = stem.clone();
    }
    if !d.aliases.contains(&stem) {
        d.aliases.push(stem.clone());
    }
    if d.upstream_ids.is_empty() {
        let upstream = match stem.as_str() {
            "hex" => "base16",
            "base_custom" => "baseCustom",
            "citrix_ctx1" => "citrixCtx1",
            "quoted_printable" => "quotedPrintable",
            "html_entity" => "htmlEntity",
            "unicode_escape" | "unicode_unescape" => "unicodeEscape",
            "rail_fence" => "railFence",
            "tap_code" => "tapCode",
            "nato_phonetic" => "natoAlphabet",
            "leetspeak" => "leetSpeak",
            "rsa_params" => "rsaParams",
            "radix_convert" => "radixConvert",
            "text_blind_watermark" => "textBlindWatermark",
            "zero_width" => "zeroWidth",
            "fullwidth_halfwidth" => "fullwidth",
            "image_diff" => "imageDiff",
            "to_hexdump" | "from_hexdump" => "hexdump",
            _ => &stem,
        };
        // Only expose real IDs in the pinned inventory; matching a name is still
        // a discovery aid, never an acceptance record.
        if upstream_ids().iter().any(|id| id == upstream) {
            d.upstream_ids.push(upstream.into());
        }
    }
    if d.description.is_empty() {
        static DESCRIPTIONS: OnceLock<std::collections::HashMap<String, String>> = OnceLock::new();
        let descriptions = DESCRIPTIONS.get_or_init(|| {
            serde_json::from_str(include_str!("descriptions.json"))
                .expect("valid builtin descriptions")
        });
        d.description = descriptions.get(&d.id).cloned().unwrap_or_else(|| {
            format!("{}。输入输出类型和可调参数见下方模块签名。", d.display_name)
        });
    }
    if d.presentation.inline_params.is_empty() {
        d.presentation.inline_params = d
            .params
            .iter()
            .filter(|p| {
                d.inputs.is_empty()
                    || match &p.widget {
                        ParamWidget::Text { multiline } => {
                            !multiline
                                && p.default.as_str().is_none_or(|s| s.len() <= 80)
                                && !["key", "iv", "pem", "alphabet", "password", "secret"]
                                    .iter()
                                    .any(|s| p.name.to_lowercase().contains(s))
                        }
                        ParamWidget::File | ParamWidget::Image => false,
                        _ => true,
                    }
            })
            .take(if d.inputs.is_empty() { usize::MAX } else { 3 })
            .map(|p| p.name.clone())
            .collect();
    }
    for port in &d.outputs {
        d.presentation
            .previews
            .entry(port.name.clone())
            .or_insert_with(|| {
                match port.port_type {
                    crate::graph::port::PortType::Bytes => "hex",
                    crate::graph::port::PortType::Json
                    | crate::graph::port::PortType::Fingerprint => "json",
                    crate::graph::port::PortType::Image => "image",
                    crate::graph::port::PortType::Candidates => "candidates",
                    crate::graph::port::PortType::Artifact => "artifact",
                    _ => "text",
                }
                .into()
            });
    }
}

pub fn matches_category(d: &NodeDescriptor, category: &str) -> bool {
    d.category == category
        || d.category_id == category
        || d.subcategory_id == category
        || d.legacy_categories.iter().any(|c| c == category)
        || categories()
            .iter()
            .any(|c| c.id == d.subcategory_id && c.name == category)
}

/// Same three tiers as the frontend search; None means no match.
pub fn search_rank(d: &NodeDescriptor, query: &str) -> Option<u8> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Some(3);
    }
    let fields = [
        &d.id,
        &d.display_name,
        &d.category,
        &d.category_id,
        &d.subcategory_id,
        &d.family,
        &d.description,
    ];
    fields
        .into_iter()
        .chain(d.aliases.iter())
        .chain(d.upstream_ids.iter())
        .chain(d.legacy_categories.iter())
        .chain(
            categories()
                .iter()
                .filter(|c| c.id == d.subcategory_id)
                .map(|c| &c.name),
        )
        .map(|s| s.to_lowercase())
        .filter_map(|s| {
            if s == q {
                Some(0)
            } else if s.starts_with(&q) {
                Some(1)
            } else if s.contains(&q) {
                Some(2)
            } else {
                None
            }
        })
        .min()
}

// Fixed EBCTFCodeBox ea2bc36 inventory, shared by catalog validation and aliases.
fn upstream_ids() -> &'static [String] {
    static IDS: OnceLock<Vec<String>> = OnceLock::new();
    IDS.get_or_init(|| {
        serde_json::from_str(include_str!("upstream_ids.json")).expect("valid pinned operation IDs")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_builtin_has_a_valid_stable_category_and_presentation() {
        for d in crate::nodes::default_registry().descriptors() {
            assert!(
                categories()
                    .iter()
                    .any(|c| c.id == d.subcategory_id
                        && c.parent_id.as_deref() == Some(&d.category_id)),
                "{}",
                d.id
            );
            assert!(!d.description.is_empty());
            assert!(
                d.upstream_ids.iter().all(|id| upstream_ids().contains(id)),
                "{}: unknown upstream ID",
                d.id
            );
            assert!(d.inputs.is_empty() || d.presentation.inline_params.len() <= 3);
            assert!(d
                .presentation
                .inline_params
                .iter()
                .all(|name| d.params.iter().any(|p| &p.name == name)));
            assert!(d
                .outputs
                .iter()
                .all(|p| d.presentation.previews.contains_key(&p.name)));
        }
    }
    #[test]
    fn old_descriptors_deserialize_and_old_categories_remain_searchable() {
        let raw = r##"{"id":"base64_decode","category":"编码/加密","displayName":"Base64 解码","color":"#fff","inputs":[],"outputs":[],"cost":"cheap"}"##;
        let mut d: NodeDescriptor = serde_json::from_str(raw).unwrap();
        enrich(&mut d);
        assert_eq!(d.subcategory_id, "encoding.base");
        assert!(matches_category(&d, "编码/加密"));
        assert_eq!(search_rank(&d, "base64"), Some(0));
    }
}
