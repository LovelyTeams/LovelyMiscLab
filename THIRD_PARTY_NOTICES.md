# 本次 EBCTFCodeBox 迁移的来源与许可

参考项目：[EBCTFCodeBox](https://github.com/Henglie/EBCTFCodeBox)，Copyright 2026 恒烈 (Henglie)。
固定参考提交：`ea2bc36dae31c97060f7f2cddad8df5518afad7b`。
上游采用 Apache License 2.0，完整文本保留在 [licenses/EBCTFCodeBox-Apache-2.0.txt](licenses/EBCTFCodeBox-Apache-2.0.txt)。

累计覆盖 143 项操作：首批 13 项、第二批 20 项原生 Rust 实现，加上 8 项既有 Base 操作核验与补齐、10 项文本传输操作、20 项数值操作、20 项地址/时间/数值报告、7 项 Base 扩展与补位隐写、5 项浮点/字符集/时间戳操作、6 项结构/对象序列化检查、11 项校验与记数操作、9 项字符集操作、6 项文本/字节传输操作，以及第十四批 8 项基础编码和字节/标识符分析操作。第六批参考 netcodec.js、netcodecExt.js、timecodec.js、timecodecExt.js、radixAll.js 和 radixExt.js；日期边界与数论资源限制差异已记录。第五批还参考 radix.js、radixExt.js 的数值契约；修正最小负数、超范围 Zeckendorf 和路径截断，差异逐项记录。没有将上游 JavaScript、WASM、本地桥或外部可执行程序加入应用运行时。算法参数改为 LovelyMiscLab 的 descriptor，原始字节经独立输出端口保留。完整操作名称、分类及来源文件映射源自上游 `src/core/lazyTables.generated.js`。

参考模块包括 `base.js`、`baseExt.js`、`text.js`、`fancy.js`、`engEncoding.js`、`encodingExt3.js`；每项的精确路径见 [完整清单](docs/ebctf-migration.json)。上游 `baseExt.js` 标注 Base58Check / Radix64 的码表来源为 WhatsInYourClipboard（ISC），并鸣谢 Leon406/ToolsFx；`engEncoding.js` 标注 Citrix CTX1 的行为参考 CyberChef（Apache-2.0）。这些来源说明在此保留。SHA-256 使用项目已有的 RustCrypto `sha2` crate。

第二批还参考 `textExt.js`、`radixExt.js`、`bitops.js`、`netcodecExt.js`、`token.js`、`byteTools.js`、`dataUri.js` 和 `bytesIo.js`。`data_uri_magic.json` 的文件签名及名称来自上游 `dataUri.js`，保留该版本的 MIME 检测语义。`base.js` 的上游来源说明同样保留：WhatsInYourClipboard（ISC），鸣谢 Leon406/ToolsFx。原有 `basex.rs` 的 CyberChef（Apache-2.0）来源不变。

第四批参考 `text.js`、`textExt.js` 和 `bytesIo.js`；`html_named.json` 是该版本 `HTML_NAMED` 的 96 项实体数据。保留上游说明：QP/UU/XX 解码行为参考 WhatsInYourClipboard（ISC），UTF-7 参考 RFC 2152，Punycode 参考 RFC 3492，Leet 符号映射参考 CyberChef。Rust 实现保留自身的边界校验及已记录的旧默认值。

`scripts/capture-upstream-vectors.mjs`、`scripts/capture-upstream-base-vectors.mjs` 和 `scripts/capture-upstream-text-vectors.mjs` 是维护者的固定版本向量捕获工具，只运行参考检出目录里的上游代码。产物是已提交的 JSON 测试数据；正常 Rust 测试不需要 Node、网络或上游检出，应用也不加载这些 JS 算法。

新增依赖 [ryu-js 1.0.3](https://github.com/boa-dev/ryu-js)（Apache-2.0 OR BSL-1.0）用于在纯 Rust 中匹配 ECMAScript 数字转字符串规则，避免 JSON/Base64url 结果因指数格式或舍入不同而改变。Zlib 复用现有 `flate2`，JSON 使用现有 `serde_json` 的键顺序与任意精度解析特性；最终序列化按上游双精度语义处理。

迁移实现针对节点接口、字节输出、参数校验和无效输入错误处理作了修改；具体兼容范围见 [迁移说明](docs/ebctf-migration.md)。本文件记录此次迁入涉及的来源，不替代现有依赖自身的许可声明。

第七批的 unicode_base_tables.json 原样保存 baseExt.js 中 B2048_11、B2048_3、B65_START、ECO_MAP、ECO_PAD4；上游注明 Unicode 表来自 WhatsInYourClipboard 的 bigBase.js（由 Kotlin 表提取）。保留 qntm Base2048、Base65536、Ecoji 与 pshihn Base69 的来源说明。新增 capture-upstream-numeric-vectors、capture-upstream-address-time-vectors、capture-upstream-unicode-base-vectors 均只用于维护者捕获固定参考数据。

第八批参考 radix.js、radixExt.js；capture-upstream-float-charset-vectors.mjs 仅捕获固定数据。直接使用已在依赖树中的 half 2.7.1（MIT OR Apache-2.0）完成标准 IEEE binary16；encoding_rs 遵循 WHATWG 编码规范。Node ICU 与浏览器码表、历史时区及上游旧半精度的差异均在验收记录和测试中明确保留。

第九批参考 serial.js 与 compress.js 的输入编码约定，迁入 Protobuf/MessagePack/CBOR 的静态解析能力；capture-upstream-serial-vectors.mjs 仅维护固定向量。实现参照各格式公开规范，补齐边界校验并提供结构化结果，未引入解释器或外部程序。

第十批继续参考 serial.js，新增 capture-upstream-object-serial-vectors.mjs 固定向量；BSON/PHP/Java 实现以数据检查为限，修复上游边界与编码缺陷。Java 对象不被实例化，PHP 引用不被执行，decimal128 仅提供原始位模式。

第十一批参考 checkdigit.js、iban.js、mayaNumerals.js、babylonianNumerals.js、egyptianNumerals.js、pipNumerals.js。checkdigit_tables.json 保存固定版本88个IBAN长度与21个BIN前缀，仅作离线参考；捕获脚本 capture-upstream-check-numeral-vectors.mjs 不参与运行时。

第十二批参考 charset.js，charset_tables.json 保留上游 EBCDIC ASCII 子集与乱码预设。测试向量捕获额外使用 text-encoding@0.7.0（Joshua Bell，Unlicense OR Apache-2.0，https://github.com/inexorabletash/text-encoding），其文件 SHA256 记录在向量中；码表和算法源自 WHATWG Encoding Standard。此独立参考仅用于维护者捕获，不随运行时或普通测试加载。Rust实现使用现有 encoding_rs；兼容策略、Node ICU差异与旧参考EUC-KR恢复差异均显式记录。

第十三批参考 fullwidth.js、jsEscape.js、bubblebabble.js、yenc.js、bech32.js 与 engEncoding.js；保留 BubbleBabble（Antti Huima）、yEnc（Jürgen Helbing）和BIP173的来源。参考向量仅在维护时运行上游，所有新运行算法仍为Rust。

第十四批参考 fengCodec.js、engEncoding.js、byteTools.js、objectIdTime.js、hexview.js、bitops.js 与 compress.js。保留上游对 DXBase64（© 2025 风之暇想，52pojie uid=243467、thread-2058510）及 Rison（Nanonid/rison）的来源说明；以纯 Rust 重新实现数据格式与报告。压缩复用 flate2，随机盐复用 getrandom，未新增运行依赖。capture-upstream-foundation-vectors.mjs 只在维护时捕获固定版本参考数据及校验 Rust 帧，不随应用运行；行为差异逐项保存在验收记录。

第十五批参考 geo.js、hamming.js、huffman.js（同一 Apache-2.0 固定版本）。保留上游对 geohash.org、Google Open Location Code、IARU Maidenhead、Snyder/USGS WGS84 投影、Richard Hamming 偶校验 SEC 及 RFC 1951 风格 canonical Huffman 分配的来源说明。全部运行实现为 Rust，未新增依赖；上游第四字母对网格扩展、普通 UTM 分区和修正边界均记录在验收源。capture-upstream-geo-coding-vectors.mjs 仅维护固定测试数据。

第十六批参考 color.js；css_named_colors.json 保留固定上游的 W3C CSS Color Module Level 3 / HTML4 147 命名色及表顺序。颜色运行实现全部为 Rust，复用已有 ECMAScript 数字格式化逻辑，没有新增依赖。capture-upstream-color-vectors.mjs 仅用于维护固定参考向量和静态色表；与完整 CSS 解析的差异及严格数值校验记录在验收源。

第十七批参考 primeGen.js；修正固定上游静态目录中 primeGen/primeTest 错指 jwtsign.js 的来源映射。Miller–Rabin 模幂/固定基、奇数候选递增和系统随机均在 Rust 实现，复用 num-bigint/num-integer/num-traits/getrandom，无新依赖。确定性界引用 Sorenson/Webster 的 [Strong Pseudoprimes to Twelve Prime Bases, Theorem 1.1](https://arxiv.org/abs/1509.00864)，不沿用上游错误的 FIPS 表引用和固定基随机误判界文案。capture-upstream-prime-vectors.mjs 仅用于维护对照数据及跨实现候选检验。

第十八批参考 difftool.js；纯 Rust 重写逐位置、经典 LCS 回溯、共同前后缀回退和报告聚合，保留固定上游的策略选择及上下文口径。新增原始字节双输入、区间 JSON、有界报告、严格 Hex 校验和取消，无新依赖。capture-upstream-diff-vectors.mjs 仅用于维护固定参考向量。
