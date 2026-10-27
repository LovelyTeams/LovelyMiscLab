# EBCTFCodeBox → LovelyMiscLab 迁移

**当前状态（2026-10-27）：v0.3.1 三平台已发布；随后恢复迁移并完成第 20 批。源码已验收 157/820 项、521 个节点；发布基线为 135/820 项、489 个节点。**

按完整清单分批用 Rust 实现。目标仓库保留现有节点图架构，不引入上游 JS、WASM、Python 或外部工具作为新增内置算法的运行依赖。

参考版本固定为 [ea2bc36](https://github.com/Henglie/EBCTFCodeBox/tree/ea2bc36dae31c97060f7f2cddad8df5518afad7b)，不是随 `main` 漂移的目标。完整 **820 项操作**见 [逐项表格](ebctf-operations.md) 和 [机器可读 JSON](ebctf-migration.json)，每项保留 ID、名称、分类、family、来源文件、目标节点和验收状态。

| 状态 | 数量 | 含义 |
|---|---:|---|
| migrated | 157 | 已实现或核验，有参数映射、固定向量和边界测试 |
| existing-review | 49 | 已有名称匹配候选，待核验参数、变体和行为 |
| pending | 614 | 待迁入，或需确认能否由现有节点组合实现 |

`existing-review` 不计作完成。一个上游操作可能拆成多个 Rust 节点，也可能由某节点的参数模式覆盖；节点数不能直接视作上游覆盖率。首批 13 项 / 24 节点保留，第二批新增 20 项 / 36 节点，第三批核验并补齐 8 项 / 16 个既有 Base 节点，第四批 10 项新增 11 个节点并补齐 8 个既有节点。第五批 20 项新增 35 个方向和报告节点。第六批 20 项新增 32 个地址、时间和分析节点。第七批 7 项新增 14 个 Unicode Base 与补位隐写节点。第八批 5 项新增 9 个浮点、字符集和时间戳节点。第九批新增 3 个二进制结构解析节点。第十批新增 3 个对象序列化检查节点。第十一批新增 21 个校验与记数节点。第十二批新增18个字符集节点。第十三批新增10个方向节点并扩展2个既有Hexdump节点。第十四批新增 8 项基础操作、10 个节点。第十五批新增 7 项坐标与纠错/压缩操作、14 个节点。第十六批新增 2 项颜色操作、3 个节点。第十七批新增 2 项素数操作、2 个节点。第十八批新增 1 项双输入差异对比节点。第十九批新增 1 项大整数计算器、覆盖 17 种数论运算。第二十批新增程序员计算器，支持 1–512 位表达式和结果接续。加上 2 个产物 I/O 节点，当前注册表总计 **521** 个节点。

人工验收源是 [ebctf-acceptance.json](ebctf-acceptance.json)，记录批次、方向、参数映射、目标节点、分类、测试及兼容差异。生成器只写派生清单和文档，校验每批不超过 20 项、映射可解析、分类与默认参数未漂移；不会按名称匹配提升完成状态。当前共 20 个批次：13 / 20 / 8 / 10 / 20 / 20 / 7 / 5 / 3 / 3 / 11 / 9 / 6 / 8 / 7 / 2 / 2 / 1 / 1 / 1 项。

## 扩展基础

- Rust `node/catalog.rs` 定义 18 个一级分类、48 个二级分组及排序/图标/颜色，注册表统一补齐旧节点元数据。桌面目录查询、浏览器生成目录、模块侧栏、模块页面、搜索菜单、帮助、AI 和 MCP 使用同一分类来源；旧分类名称保留为兼容别名。
- 搜索覆盖中英文名称、节点 ID、上游 ID、别名、说明、家族与分类，按精确/前缀/包含排序。侧栏默认分类折叠，分组先显示 12 项、每次再加载 24 项；模块页面每页 50 项，支持二级分组。
- `NodeDescriptor` 新增可选元数据，旧 descriptor 使用默认值。普通节点最多显示 3 个常用参数，密钥、长码表、多行配置和高级参数进入侧栏，源节点仍可直接输入。
- 参数组件和结果组件由画布、侧栏、独立调用复用；文本/Hex/JSON 树/表格/候选列表/图片和文件产物使用有界预览，运行记录保存摘要。错误和日志也限制显示/保存长度。
- 独立调用支持文本、Hex/Base64、文件、数值、布尔、JSON、列表、图片和产物 UUID；未启用的可选端口不发送。示例保持可编辑，按声明格式转成真实 `PortValue`。
- 独立调用接入统一流式执行和前台任务锁；关闭或取消时取消所属任务，任务 ID 迟到也会取消；迟到事件与结果不回填。结果只留在调用窗口，不写入画布。
- 产物存放在桌面应用数据目录的 `artifacts` 文件夹，以 UUID 定位；提供文件名/MIME/长度/SHA-256、按需读取和明确导出。单产物上限 128 MiB，单次读取上限 1 MiB，界面媒体预览与直接上传上限 16 MiB。`artifact_store` 保存字节，`artifact_read` 显式恢复字节，端口类型不会暗中转换。
- 日志和本地历史不保存图片或原始字节副本，只保留摘要和产物引用。大文件需通过画布文件导入节点处理；导出不会静默覆盖现有文件。

## 第一批：常用编码缺口

| 上游操作 | Rust 节点 | 参数 / 验收范围 |
|---|---|---|
| base36 | base36_encode / decode | 默认 0–9a–z，36 字符自定义 alphabet，保留前导零 |
| base91 | base91_encode / decode | 标准 basE91、91 字符自定义 alphabet；strip 允许显式忽略非法字符 |
| base100 | base100_encode / decode | 每字节映射 U+1F3F7 + b |
| base58check | base58check_encode / decode | 自定义 alphabet，双 SHA-256 的 4 字节校验；不自动添加地址版本字节 |
| baseCustom | base_custom_encode / decode | 唯一字符码表、默认 0–9A–F、任意进制，保留前导零 |
| radix64 | radix64_encode / decode | crypt 字母表 `./A–Za–z0–9`，无填充位字符 |
| modhex | modhex_encode / decode | none/space/colon/comma 输出分隔符，解码忽略常见分隔符且大小写兼容 |
| crockford32 | crockford32_encode / decode | check 校验位、dashGroup 分组、O/I/L 容错；分组控件范围 0–128 |
| uuencode / xxencode | uuencode_encode / decode、xxencode_encode / decode | 每行 45 字节；默认 begin/end 信封，bare 兼容模式只输出数据行 |
| citrixCtx1 | citrix_ctx1_encode / decode | UTF-16LE 链式异或，A–P 编码，中文和代理对测试 |
| rot5 / rot18 | rot5、rot18 | 数字轮转与字母轮转；同操作执行两次还原 |

上游参数 `dict` 对应节点 `alphabet`；`compat` 对应 `bare`。输入字节接 `data`，解码字符串接 `text`。字节解码器同时输出 `text` 预览和完整 `bytes`；无效 UTF-8 的文本预览不适合继续二进制运算，应连接字节端口。Citrix CTX1 与 ROT 节点直接接文本。

兼容目标是合法数据和明确参数的处理能力。Rust 解码器对截断、重复码表、无效校验和非零尾部补位报错，不复制上游某些宽松错误恢复；Citrix CTX1 拒绝无效 UTF-16。自定义码表按 Unicode 标量字符计数，避免 JavaScript UTF-16 码元切分问题；码表包含空白时保留输入首尾空白。上游自动识别评分、课程卡片和一把梭调度器尚未迁入，不能由单项编解码完成状态推断这些能力已完成。

## 第二批：传输、位与字节、结构解析（20 项）

| 上游操作 | Rust 节点 | 主要参数 / 输出 |
|---|---|---|
| z85、base85ipv6 | z85_encode/decode、base85ipv6_encode/decode | dict；IPv6 编码 16 字节，RFC 1924 整数模式 |
| base64dict、multilineBase64 | base64_dict_encode/decode、multiline_base64_encode/decode | key、ignoreUnknown；lineLen、skipInvalid，按方向显示 |
| base64decompress | base64_zlib_encode/decode | 纯 Rust Zlib、maxBytes 解压上限、进度与取消 |
| netbios、caretMdecode、httpBasicAuth | netbios_encode/decode、caret_m_encode/decode、http_basic_auth_encode/decode | NetBIOS A–P、Caret/M 记法、Basic 头；完整 bytes |
| bitReverse、bitRotate | bit_reverse_encode/decode、bit_rotate_encode/decode | 字节内位反转；dir、bits 循环移位 |
| byteSwap、byteReverse、hexReverse | byte_swap_encode/decode、byte_reverse_encode/decode、hex_reverse_encode/decode | group / mode；整串反转；半字节互换 |
| urlQueryParse、cookieParse、asciiControl、splitHex | url_query_parse、cookie_parse、ascii_control、split_hex | 重复键与属性表、控制字符 mode、Hex 分组报告 |
| varint | varint_encode/decode | signed ZigZag；任意精度整数、bytes、consumed、remaining |
| b64urlJson | b64url_json_encode/decode | ECMAScript JSON 数字格式及整数键排序、JSON/bytes 输出 |
| dataUriParse | data_uri_encode/decode | mime/base64、文件魔数及 MIME 不匹配提示、metadata/bytes/artifact |

固定参考版本捕获 **177** 条向量，另测全部 256 字节、短组、错误输入、压缩截断/校验、解压大小限制、执行中取消和真实产物读取。36 个新节点的示例在独立调用与图执行两种入口比较输出。

已记录的接口调整：Z85/IPv6 的自定义字典同时用于编码和解码；NetBIOS 额外接受空解码；Caret/M 保留上游 Latin-1 低字节和字面 `^` / `M-` 歧义；压缩实现的压缩字节可能不同但解压内容一致；Data URI 报告适配节点输出，完整文件另存产物。JSON 只接受有效 Unicode，孤立 UTF-16 代理项不作为合法 JSON 处理。每项参数和差异详见人工验收记录。

## 第三批：核验 Base 家族（8 项）

| 操作 | 补齐内容与兼容映射 |
|---|---|
| base16 | 保留 hex 节点的 text 端口；支持字节、自定义码表、大小写/空格，解码输出 bytes。上游 upper 默认 true 需显式映射，旧默认小写保持。 |
| base32 | 标准/Hex/Crockford/z-base-32/自定义，padding、大小写与 O/I/L 容错。严格模式拒绝截断和非零补位。 |
| base45 | 自定义码表、空格保真；修复单字符尾组、双字符组超过 255、三字符组超过 65535。 |
| base58 | Bitcoin/Flickr/Ripple/自定义，唯一性校验、逐字字典与旧范围语法并存；大输入进度和取消。 |
| base62 | 新增 preserveZeros=true 匹配上游，旧默认 false 保持；逐字字典/严格模式，大输入进度和取消。 |
| base64 | 旧 text 端口名保留，编码扩展为 Any，解码新增 bytes；标准/URL/自定义与 padding，URL 替换可和自定义模式组合。 |
| base85 | Ascii85 的 delim=!compat；修复短零分组错误，拒绝单字符尾组和 32 位溢出；旧变体保留。 |
| base92 | 91 字符自定义码表加 ~ 空标记；emptyMarker=true 对应上游，旧默认空串保持。 |

**314** 条上游向量覆盖合法样例、空文本、Unicode、前导零、全部字节及关键变体；另有旧默认值、错误输入、图/独立执行一致与取消测试。保留所有旧 ID 和端口名，新增参数有默认值。8 项从原有 63 个名称候选中完成核验，其余 55 项继续保留待核验状态。

## 第四批：文本传输与字符表示（10 项）

| 操作 | 节点与参数映射 |
|---|---|
| url | 旧 url_encode/decode；standard/full/plus 模式，编码接收文本或字节，解码新增 bytes。上游默认映射 mode=standard、plus/trim/strict=true；旧默认保留。 |
| htmlEntity | 旧 html_entity_encode/decode；补齐 96 项命名实体，namedApostrophe=true 对应上游 &apos;，旧默认 &#39; 保留。 |
| unicodeEscape | 旧 unicode_escape/unescape；补齐 uXXXX/U+/hex 与大写开关，修复 emoji 代理对丢失；上游映射 mode=全部、upper=true。 |
| quotedPrintable | 旧 quoted_printable_encode/decode；原始字节模式精确保留 CR/LF，邮件文本模式维持旧默认；修复异常 Unicode 输入越界。 |
| utf7 | utf7_encode/decode；UTF-16BE 移位序列、直接字符集、显式/隐式终止、+- 字面加号。 |
| punycode | punycode_encode/decode；RFC 3492，按点号分段，xn-- 前缀，进度与取消；输入 1 MiB、每段 4096 字符上限。 |
| jsHex | js_hex_encode/decode；nonascii/all，反斜杠也编码，字节保真；未转义字符可选择上游 UTF-16 低字节或 UTF-8。 |
| mixHexOctBin | mixed_radix_decode；0x/0b/0o/0d 连写或分隔输入，报告、拼接文本、表格、错误项分别输出；strict=false 对应上游。 |
| leetSpeak | leet_speak_encode/decode；完整符号表、最长 token 优先，旧 leetspeak 节点精简映射保持。 |
| natoAlphabet | nato_alphabet_encode/decode；Juliet/X-ray、数字、(space)，保留未知 token；旧 nato_phonetic 行为保持。 |

**214** 条固定上游向量涵盖这 10 项的全部方向与关键参数。附加测试覆盖 256 字节、不同长度、旧默认值、无效代理项、截断 UTF-7、Punycode 溢出与取消、混排结果连接下游；所有带示例节点继续比较图执行和独立执行结果。新增节点拥有用途、参数帮助、分类、上游别名和可编辑样例。

兼容差异已逐项记录：非法 UTF-16 代理项不静默丢弃；QP 不复制半有效 Hex 转义的 parseInt 容错；UTF-7 不接受截断码元与非零补位；Punycode 不进行 IDNA 规范化或 DNS 请求；混排整数使用精确 u64，错误项单独输出。Leet 与 NATO 的大小写丢失和歧义与上游相同。累计原有候选中 13 项完成核验，其余 50 项继续保留待核验状态。

## 第五批：精确整数、数值表示与分析（20 项）

新增 35 个节点：进制互转、字节进制、BCD、二进制补零、混合进制字符、数字串 ASCII 分割、UTF-16 字符偏移、原反补码分析与解释、补零对齐、GCD/LCM、斐波那契替换、负进制、平衡三进制、阶乘进制、Zeckendorf、罗马数字、中文逐位数字、连分数和 Stern-Brocot。原有 radix_convert、roman_numeral、number_theory 保持兼容。

405 条上游向量逐项比对；另测全部 256 字节、空输入、大整数与负数、超 2^256 的 Zeckendorf、错误与资源边界、运行中取消、结构化候选接续和全部示例的图/独立执行一致性。字符偏移按上游首个 UTF-16 码元处理，孤立代理项的原始值保存在 bytes/rows.utf16le；文本预览使用替换符。

明确修正上游缺陷：最小负数不再生成超位宽原反补码，Stern-Brocot 超步数报错而不静默截断，Zeckendorf 动态扩展高位。严格完整整数解析不复制 parseInt 的部分有效输入容错；阶乘解码 strict=false 提供上游宽松加权模式。逐项差异与默认值在人工验收记录中。

第 5 批检查：核心 328、应用/MCP 20、前端 26 项通过；严格 Clippy、生产构建、UTF-8、Rust/浏览器目录一致性与本批 Rust 格式检查通过。

## 第六批：地址、时间与数值报告（20 项）

新增 32 个节点：IPv4 整数、IPv6 压缩/展开、MAC 格式、CIDR、User-Agent、Magnet、Unix、FILETIME、HFS+、Cocoa、DOS、汉字日期、时区转换、儒略日、Excel 日期、Chrome 时间、雪花 ID、素数试除、Collatz 和一键多进制。每项有参数帮助、分类与上下游可接续结果；地址和链接只进行离线解析。

469 条上游向量涵盖全部方向和主要参数。额外检查早期年份、毫秒保真、/0、/31、/32、Excel 虚构闰日、DOS 年份与日期溢出、候选列表、分解未完成状态和进度取消。FILETIME 与 Chrome 值使用精确整数，ISO 主结果保持上游秒显示，data/毫秒输出保留更细信息。

修正并记录 JS 日期的错误世纪推断、无效日期自动滚动、DOS 超范围打包与 Excel 序列 60 的小数日漏洞。儒略日保留上游改历分支，并在说明中写明古代日期双向不对称。素数试除达到限制会输出未完成余数，不把它认定为素数；Collatz 输出完整序列和到达 1 的状态。

第 6 批检查：核心 333、应用/MCP 20、前端 26 项通过；严格 Clippy、生产构建、UTF-8、目录一致性及本批 Rust 格式检查通过。

## 第七批：Unicode Base 与补位隐写（7 项）

新增任意进制、Base69、Base2048、Base65536、Ecoji、Base32 补位隐写和 Base64 补位隐写的 14 个方向节点。旧 base64_stego 提取节点保持原有端口。固定 Unicode 码表逐项核对上游并保存来源。

276 条参考向量覆盖 Unicode、BOM、空输入、二进制、关键参数与短尾。额外检查全部 256 字节、全部 65536 个 Base65536 字节对、码表唯一性、非末尾短组、padding、两方向执行中取消；所有示例继续比较图执行与独立执行输出。

默认严格拒绝未知字符、错误短组和补位；strict=false 保留上游 Base2048 跳过未知字符、Base65536 取未知块低字节、Ecoji 忽略短尾和未知字符等行为。Base69 支持严格的历史 7= 尾形态。新增隐写节点输出行明细、字节和未满字节尾位，避免默默丢失分析依据。

第 7 批检查：核心 338、应用/MCP 20、前端 26 项通过；严格 Clippy、生产构建、UTF-8、目录一致性及本批 Rust 格式检查通过。

## 第八批：浮点、字符集与时间戳（5 项）

新增 IEEE754 编解码、浮点进制两方向、多字符集编解码分析、时间戳自动/两方向报告，共 9 个节点。350 条固定向量覆盖全部精度与参数，额外验证正确 IEEE 半精度、真实 GBK/Big5/日文/Windows-1252 字节、BOM、全部字节、明确时区和执行中取消。

默认 halfMode=ieee 使用标准最近偶数舍入；upstream 模式复现固定版本的半精度缺陷。字符集默认实际编码，legacyReport=true 可复现原未实现报告。解码按 [WHATWG Encoding](https://encoding.spec.whatwg.org/) 实现；参考捕获所用 Node ICU 的 5 个输入存在具体码表差异，测试保留原文并逐行断言标准结果。时间戳使用固定偏移，不套用历史上海 LMT 或 DST；错误日期严格拒绝。完整差异保存在人工验收记录。

第 8 批检查：核心 343、应用/MCP 20、前端 26 项通过；严格 Clippy、生产构建、UTF-8、434 项目录一致性及本批 Rust 格式检查通过。

## 第九批：二进制结构解析（3 项）

新增 Protobuf Wire、MessagePack 和 CBOR，归入文本与数据 / 序列化。每项支持自动识别、Hex、Base64、UTF-8 和直接字节输入，输出结构树、报告、已消费长度、剩余字节、完整状态和错误。64 位整数使用精确文本；结构树保留重复键、非文本键、原始字符串字节与偏移。

500 条固定向量逐字匹配；额外检查全部字节、每个截断前缀、错误长度、非法 CBOR break/不定长块、Protobuf 部分字段、嵌套/项数限制与执行中取消。CBOR 修复上游 64 位长度与 simple 值解析；MessagePack 修复缺失数据补零；Protobuf 拒绝超范围 varint/字段号。依据 [CBOR RFC 8949](https://www.rfc-editor.org/rfc/rfc8949.html)、[MessagePack 规范](https://github.com/msgpack/msgpack/blob/master/spec.md) 和 [Protobuf wire 规范](https://protobuf.dev/programming-guides/encoding/)。

第 9 批检查：核心 348、应用/MCP 20、前端 26 项通过；严格 Clippy、生产构建、UTF-8、437 项目录一致性与本批 Rust 格式检查通过。

## 第十批：对象序列化检查（3 项）

新增 BSON、PHP serialize、Java 序列化静态识别，统一接入结构树、字节残余和完整解析状态。BSON 正确消费嵌套文档终止符、继续读取兄弟字段；PHP 按字节计数，保留 Unicode、任意原始字符串、大整数与嵌套元数据；Java 正确读取 modified UTF-8 与 flags，未知/仅扫描的对象标记显式不算完整解析。

249 条固定参考向量，BSON 最后 NUL 的游标差异精确断言。额外验证所有截断前缀、全部字节、PHP 中文/emoji、大整数、Java NUL/代理对/孤立代理、错误长度、深度限制和执行中取消。依据 [BSON 规范](https://bsonspec.org/spec.html)、[PHP serialize 手册](https://www.php.net/manual/en/function.serialize.php) 与 [Java 序列化协议](https://docs.oracle.com/en/java/javase/17/docs/specs/serialization/protocol.html)。这些节点只解析数据，不实例化或执行对象。

第 10 批检查：核心 353、应用/MCP 20、前端 26 项通过；严格 Clippy、生产构建、UTF-8、440 项目录一致性及本批 Rust 格式检查通过。

## 第十一批：校验位、IBAN/BIN 与历史记数（11 项）

Luhn、ISBN-10/13、EAN-13、身份证校验位、UPC-A、IBAN 分为计算/验证节点，分别输出校验位、完整号码和布尔结果；BIN 输出离线参考结构。玛雅、巴比伦、埃及、骰子与骨牌记数按方向拆分，支持精确大整数、模式/形态和分隔符。

749 条上游向量覆盖全部方向和参数，88 个 IBAN 国家长度及21个 BIN 前缀。额外检查所有骰子/骨牌和记数符号、大整数、位边界、校验位变异、错误输入和取消。IBAN/BIN 的表仅对应固定版本；身份证节点只检查校验位。

第 11 批检查：核心 358、应用/MCP 20、前端 26 项通过；严格 Clippy、生产构建、UTF-8、461 项目录一致性及本批 Rust 格式检查通过。浏览器检查上游ID搜索、分类、独立调用参数帮助与示例填充；没有将模拟后端当成原生算法执行。

## 第十二批：字符集转换、区位码与乱码修复（9 项）

新增18个方向节点，覆盖 GBK/GB2312/GB18030、Big5、Shift-JIS、EUC-KR、14种单字节标签、EBCDIC上游ASCII子集、UTF-16、GB2312区位码、9种乱码错配与自定义组合。默认 standard 编码在不可表示时明确报错；upstream 复现原反向表与UTF-8混合回退。输出转换字节、原始字节和有损状态，支持取消。

1003条固定向量使用独立 text-encoding@0.7.0 参考捕获，文件哈希固定，避开Node ICU与浏览器码表差异。旧参考EUC-KR吞掉无效前导后ASCII的一个差异依据[WHATWG Encoding](https://encoding.spec.whatwg.org/#euc-kr-decoder)单独断言，其他1002条精确比较。另测全部256字节、原始输入保真、标准GB18030四字节、BOM/孤立代理、乱码损失、错误参数、上限与取消。

第 12 批检查：核心364、应用/MCP20、前端26项通过；严格Clippy、生产构建、UTF-8、479项目录一致性与本批Rust格式检查通过。

## 第十三批：文本与字节传输（6 项）

新增全角/半角、JS escape、BubbleBabble、yEnc、Bech32方向节点，并扩展现有Hexdump节点。旧ID、端口和默认值保持，xxd为显式模式；解码完整字节单独输出，孤立UTF-16码元可显式保留。Bech32只处理通用载荷，不推断见证版本。

389条固定向量逐项比对，另测全部字节、短尾、Unicode、BOM、标准校验向量与逐字符变异、重复行扩张边界和取消。严格xxd修复上游-C双分组与终止偏移，宽松模式保留参考行为。

第13批检查：核心370、应用/MCP20、前端26项通过；严格Clippy、生产构建、UTF-8与489项目录生成通过。用户要求暂停后，仅收尾本批记录、替换应用图标和准备桌面测试版，未启动新迁移批次。

## 后续批次与非操作功能

1. 核验剩余 49 个现有候选，补进制数值与数据格式；每项补参数映射和向量后才提升为 migrated。
2. 扩展古典密码、现代密码、哈希与分析算法；逐项评估 Rust crate 和跨实现向量。
3. 文件/归档/文档/PCAP 取证与图像、音视频、隐写；样本文件和二进制产物需要端到端验收。
4. 需要 WASM、Windows 桥或外部工具的操作逐项用原生 Rust 替代；无法保持纯 Rust 的能力保持 pending，并记录具体缺口。
5. 组合节点待迁移：自动链式解码、深度/强度/算法池/目标特征配置、密钥+密文组合尝试、文件自动分析、候选选择与下游接续。现有单节点、图执行、MCP、产物接口可复用，但这些组合能力尚未完成。本次范围是操作能力及已确认组合节点；上游 PWA、课程卡片、GUI 桥等产品形态不计入操作完成状态。

## AI 与节点交互验收

- 规划、解释、规划修复均通过 AI 助手入口。生成计划只返回数据；前后端校验节点、参数、端口类型、必填输入、重复输入及环路。
- 普通规划与规划修复显示说明、全部节点、完整参数和连线，明确展示追加/替换选择。确认前不改变画布；修改输入或画布后需重新生成计划。
- 普通规划确认后只搭建、执行已展示的子图，使用普通 Rust 图执行器。自主解题从 v0.3.1 起一次点击开始，画布直接显示每轮判断、检索和计划，在限定轮数/时限内自动修改或继续拼接。每轮搭建可撤销，取消保留已经搭建的节点。
- AI 临时网络与响应异常最多自动重试两次，仍失败时可在画布手动继续当前请求；已有图和证据保留，取消与总时限仍生效。模块检索支持中英文组合、参数模式和真实目录反馈，未知能力不伪装成已命中。
- 整图、单节点和运行到节点共用运行状态。取消早于任务 ID 返回时仍能取消；参数变化后旧结果不回填；暂停不会触发积压的实时重跑。
- 批量删除、拖动与复制粘贴保留合理撤销边界；禁用状态随粘贴保留，删除/更换连线清理受影响的旧输出。
- Galgame 入口、前端状态/组件、后端命令和专属素材已移除。

## 复现

```powershell
pnpm generate:catalog
pnpm check:catalog # Rust、浏览器节点与分类目录一致性
pnpm generate:migration # 默认读取 .git/reference/EBCTFCodeBox
# 或 node scripts/generate-migration.mjs D:/path/to/EBCTFCodeBox
pnpm test:frontend
cargo test -p misclab-core
cargo test -p misclab-app --features mcp
cargo clippy --all-targets -- -D warnings
pnpm check:utf8
pnpm build
```

前端测试直接调用真实 store/runner，模拟 Tauri 传输与延迟事件，覆盖取消竞态、只执行确认子图、一次撤销、旧计划拒绝等行为。浏览器预览读取 Rust 导出目录，可检查布局和节点操作，但不调用模型或执行后端；真实模型响应与桌面 WebView 的联网端到端测试需使用已配置模型的桌面应用。

来源与许可见 [第三方声明](../THIRD_PARTY_NOTICES.md)。

## 迁移暂停前验证记录（2026-10-27）

- Rust 核心：370 通过，3 个原有耗时 bkcrack 测试按默认配置忽略。
- Tauri 应用（包含 MCP）：20 通过，包含分类层级/计数、旧分类查询、上游别名和 AI 目录签名。
- 前端工作流：26 通过，包含节点级/任务级错误、任务启动前取消、旧结果丢弃、类型化输入、独立调用锁和示例填入。
- `cargo clippy --all-targets -- -D warnings`、`pnpm build`、`pnpm check:utf8`、Git 差异空白检查通过。
- 第四批浏览器验证：344 个模块按 50 项分页，分类与二级分组正确；上游 ID 可搜索 multilineBase64、mixHexOctBin；独立调用示例可编辑，奇数位 Hex 在执行前报错；新增混排节点可加入画布、内联参数同步侧栏，两次撤销恢复测试前四节点流程。先前批量删除/一次撤销、Base36 添加、AI 计划入口验证保持有效。
- 所有本次改动的 Rust 文件已格式化并逐文件检查。迁移开始时记录了 164 个原有格式差异文件，本次仅格式化涉及的文件，未把其余文件整体重新排版混入迁移。
- 此前迁移阶段的计划确认与执行隔离使用模拟传输回归测试，Rust 后端计划解析使用离线测试。

## v0.3.1 交互回归补充（2026-10-27）

真实 DeepSeek 联调完成 ZIP、PNG 绿色通道 LSB、AES、Base64、Hex、ROT13 的五轮解题流程，最终 14 个节点、15 条连线。独立 Rust 执行器重新运行导出流程，14 个节点全部完成且答案精确匹配；密钥和 IV 来自输入文件，没有写成参数常量。另用用户提供的未知密文完成维吉尼亚分析，重新加密后与 131 个原始字符完全一致。

真实模型流程中分别注入一次 HTTP 503 和连续三次响应体中断，验证自动重试、手动继续、原有画布和执行证据保留；故障注入只存在于显式启用的测试桥，不修改模型配置。其他取消、迟到事件、鉴权失败和模块缺失场景由前后端回归覆盖。

## 第 14 批：基础编码与字节/标识符分析（2026-10-27）

本批在 v0.3.1 发布之后实现，属于源码开发增量。8 项操作全部通过验收，新增 10 个节点，现有节点 ID、端口和默认值保持兼容。

| 上游操作 | Rust 节点 | 分类与接续结果 |
|---|---|---|
| dxBase64 | `dx_base64_encode` / `dx_base64_decode` | Base；解码同时给文本预览与完整字节 |
| rison | `rison_encode` / `rison_decode` | 序列化；value/object/array/uri，解码附 JSON |
| unixPerms | `unix_permissions` | 位与字节；符号、八进制、特殊位与 chmod 说明 |
| uuidParse | `uuid_parse` | 时间日期；版本/变体及 v1/v7 时间字段 |
| objectIdTime | `object_id_time` | 时间日期；UTC、随机值和计数器 |
| hexRange | `hex_range` | 位与字节；多进制报告、区间 bytes 与 JSON |
| hexStats | `hex_stats` | 统计分析；256 桶、密度图、全局/窗口熵与 JSON |
| bitPlaneExtract | `bit_plane_extract` | 位与字节；MSB/LSB 和全部平面，结构化比特串 |

576 条固定参考向量逐项比较，另验证全部 4096 种 UNIX 权限的符号往返；16 个实际 Rust DX 帧通过上游解码器和独立 zlib 验证。覆盖空输入、Unicode、全字节、所有声明方向和参数模式、错误输入、深度/大小限制、解压膨胀、执行中取消及图/独立调用结果一致性。DX 允许不同合法 deflate 帧；新增固定盐仅用于可复现配置，默认仍为随机盐。Rison 把原型键当普通数据，拒绝孤立 UTF-16 代理；详细边界与差异见人工验收记录。

模块分类、中文/英文别名、上游 ID、参数帮助、示例及文本/Hex/JSON 预览均由 descriptor 提供。前端回归验证检索、连线、参数修改和撤销、独立调用不改画布；MCP 分类检索与真实 HTTP 字节输入验证通过，并修复标准字节数组在 MCP 入口被拒绝的问题。

本批检查：核心 380 项通过（3 项既有耗时测试忽略），应用/MCP 34 项通过（1 项显式真实模型测试忽略），前端 77 项通过；严格 Clippy、生产前端构建、UTF-8、499 项目录一致性及修改 Rust 文件格式检查通过。原有全仓格式差异继续单独追踪。

参考向量可用 `node scripts/capture-upstream-foundation-vectors.mjs` 重新捕获；普通 Rust 测试直接读取已提交向量，无需上游目录、Node 或网络。维护者可设置 `MISCLAB_DX_REFERENCE_EXPORT` 后运行 `cargo test -p misclab-core --test migration_foundation`，再用捕获脚本的 `--verify-rust <导出路径>` 检查跨实现互通。

## 第 15 批：地理编码、海明纠错与 Huffman（2026-10-27）

7 项操作映射 14 个方向节点，新增内容仍是 v0.3.1 发布后的源码增量。

| 上游操作 | Rust 节点 | 分类与节点内容 |
|---|---|---|
| geoDms | `geo_dms_encode` / `geo_dms_decode` | 地址与单位；DMS / 十进制度、文本与坐标数值 |
| geoHash | `geo_hash_encode` / `geo_hash_decode` | 地址与单位；1–12 位精度、中心点与边界 JSON |
| geoPlusCode | `geo_plus_code_encode` / `geo_plus_code_decode` | 地址与单位；2–15 显著位全码、填充与网格细分 |
| geoMaidenhead | `geo_maidenhead_encode` / `geo_maidenhead_decode` | 地址与单位；2/3/4 对、上游字母扩展、中心点 |
| geoUtm | `geo_utm_encode` / `geo_utm_decode` | 地址与单位；WGS84 / Snyder、普通 6° 分区 |
| hammingCode | `hamming_code_encode` / `hamming_code_decode` | 位与字节；k、偶校验 SEC、补位/纠错位置 JSON |
| huffmanCodec | `huffman_encode` / `huffman_decode` | 压缩；频率/用户码表、完整位流、位数及码表接续 |

Huffman 的 `packet` 可直接接解码的 `data`，也可同时连 `bytes`、`code_table`、`bit_length`；文本报告仍保留上游格式，完整数据从专用端口取得。权重表和码表放侧栏，画布只显示模式和位数。解码保留完整字节，UTF-8 预览与 JSON 详情分开提供。坐标解码新增数值端口，避免 6 位小数文本造成精度损失。

820 条固定上游向量覆盖所有方向及公开选项。额外验证海明码每一位单错纠正、补零与截断尾位，Huffman 全 256 字节和 Unicode/BOM、255 层树、报告截断后的完整端口、图接续和独立执行一致、运行中取消及无效输入/资源上限。修正上游 UTM 180° 生成第 61 区、Maidenhead 上界无效网格、Huffman 深树数值溢出；第 4 对 Maidenhead 字母扩展和无特殊 UTM 分区仍按固定上游保留，差异见人工验收源。

检查通过：核心 387 项（3 项既有耗时测试忽略）、应用/MCP 34 项（1 项显式真实模型测试忽略）、前端 79 项；真实 MCP HTTP 接口收到 JSON Huffman 数据包并正确解码。严格 Clippy、生产前端构建、UTF-8、513 项目录一致性和修改 Rust 文件格式检查均通过。旧 143 项人工记录逐项比较未变化，生成器不覆盖人工状态。

向量维护命令：`node scripts/capture-upstream-geo-coding-vectors.mjs`；日常验收：`cargo test -p misclab-core --test migration_geo_coding --locked`。应用不加载参考 JavaScript，也未引入新运行依赖。

## 第 16 批：颜色互转与分析（2026-10-27）

上游 color 映射为 `color_convert_encode` / `color_convert_decode`，colorInfo 映射为 `color_info`，归入进制与数值 / 地址与单位。正反方向共用 from/to/upper 三个内联参数，覆盖七种格式及全部 147 个 CSS3 命名色。结果提供兼容文本、RGB 三字节、三个数值分量和颜色详情 JSON；可直接接字节分析等下游模块。

保留上游 RGB 数值启发式及自动识别优先级，并在参数帮助中注明不是浏览器 CSS 解析器；无透明度。明确格式可避免 3/6 位数字按 Hex 识别的歧义。严格拒绝非有限数值、数值垃圾后缀和混入非法字段；Hex/HSL/HSV/CMYK 数字格式复用已有 Rust 精确舍入。

3932 条固定上游向量覆盖所有命名色、格式、方向及大小写；额外检查 RGB 字节/数值/JSON 一致、重复命名顺序、空输入/Unicode/错误输入、大小限制、取消、图与独立调用接续、前端参数/撤销和真实 MCP HTTP 调用。核心 391 项通过（3 项既有耗时测试忽略）、应用/MCP 34 项通过（1 项显式真实模型测试忽略）、前端 80 项通过。严格 Clippy、生产前端构建、UTF-8、516 项目录一致性及修改 Rust 文件格式检查通过；前 150 项验收记录保持不变。

全量验证首次遇到 Windows PDB 链接错误，正常取消构建后使用 Cargo 清理开发构建缓存并以单任务重建，最终重跑通过。向量维护命令为 `node scripts/capture-upstream-color-vectors.mjs`，常规验收为 `cargo test -p misclab-core --test migration_colors --locked`，无需上游 JS 或网络。

## 第 17 批：素性检验与候选生成（2026-10-27）

primeGen → `prime_generate`，primeTest → `prime_test`，均归数论。生成器无需文本输入，内联位数和数量；检验器可接 text 或在侧栏填写大整数 n，内联轮数。生成给出十进制文本、候选列表和依据 JSON；检验给出报告、布尔通过状态、位长及确定性依据，可继续接控制节点。

保留固定上游的 13 个固定质数基及大数指定轮数规则，修正成功大数的“素数”措辞为“可能素数”，不套用随机基误判概率。确定性上界来自 [Sorenson/Webster 的定理 1.1](https://arxiv.org/abs/1509.00864)。上游静态清单的两个源路径错误已在人工记录中修正为实际 primeGen.js。运行使用系统 CSPRNG，模幂内部检查取消；不引入新依赖。

207 条固定参考样例和 9 个上游生成候选通过，另验 0–999 独立试除、确定性边界强伪素数的已知因子、所有 64 个轮数、2–1024 位生成、模幂/生成中取消、范围和输入错误。18 个实际 Rust 候选通过上游独立运行交叉检验。检验报告文案修正有明确差异记录，未伪装成上游逐字相同。

检查通过：核心 397 项（3 项既有耗时测试忽略）、应用/MCP 34 项（1 项显式真实模型测试忽略）、前端 81 项，严格 Clippy、生产前端构建、UTF-8、518 项目录一致性和修改 Rust 文件格式检查通过。前 152 项人工记录不变。维护命令：`node scripts/capture-upstream-prime-vectors.mjs`；设置 `MISCLAB_PRIME_REFERENCE_EXPORT` 后跑 `cargo test -p misclab-core --test migration_primes`，可用脚本 `--verify-rust <JSON 路径>` 核验实际生成结果。

## 第 18 批：双输入文本/字节差异对比（2026-10-27）

上游 diffTool 映射为 `diff_tool`，归文本与数据 / 文本处理。新增 a、b 两个可连线输入，b 优先于侧栏 textB；内联 mode/inputEnc/context。支持原始 Bytes 或编码文本，auto 以 A 统一解释两边，line 按 UTF-8 和 LF/CRLF 分行。报告附 equal 布尔、结构化区间/统计/策略 JSON，以及完整 A/B 字节用于下游接续。

保持等长逐位、非等长 LCS、超过 200 万格采用共同前后缀回退的策略选择。修正非法 Hex 静默变零；报告上限 1 MiB、JSON 上限 16384 连续段，明确标出截断并保留完整统计和输入，每段上限 256 KiB。并非所有策略都求最小编辑距离，节点说明已明确。

150 条固定上游对照覆盖所有方向/模式/编码/上下文和策略；另验全字节与 Unicode、CRLF、双输入覆盖参数、区间坐标、2 万差异块有界显示、5 万行回退、错误/大小限制、各策略运行中取消和图/独立调用一致。前端双输入连线、独立调用不改画布、上下文编辑撤销，以及真实 MCP HTTP 双字节输入均通过。

检查通过：核心 403 项（3 项既有耗时测试忽略）、应用/MCP 34 项（1 项显式真实模型测试忽略）、前端 82 项；严格 Clippy、生产前端构建、UTF-8、519 项目录一致性和修改 Rust 文件格式检查通过。前 154 项人工记录不变。维护命令：`node scripts/capture-upstream-diff-vectors.mjs`；常规验收：`cargo test -p misclab-core --test migration_diff --locked`。

## 第 19 批：精确大整数计算与可取消分解（2026-10-27）

上游 bigCalc 映射为 `big_integer_calc`，归进制与数值 / 数论。17 种运算覆盖四则、整数幂、截断余、模幂/模逆、gcd/extgcd/lcm、素性、邻素数、素因子分解、整数开方和位长。a/b/m 均可连线并覆盖侧栏参数，内联显示运算；输出精确文本与 JSON，所有大数不经过浮点转换。有效默认值为 a=42、b=17、m=101。

分解采用小素数试除、Miller–Rabin 和系统随机初值的 Pollard–Brent；新增统一 3145728 步预算，正向、回退及递归分支都计数，耗尽保留剩余项。确定性界以外只标可能素数，并分开说明是否拆完和是否已证明；大数输入最多 4096 二进制位，整数幂估算结果最多 100 万位，耗时循环支持进度/取消。

338 条上游对照覆盖全部模式与错误；额外验证 525 组 Bézout 恒等式、负数除法/余数、4096 位开方边界、>2^53 文本精度、分解乘积与预算耗尽、强伪素数边界、运行中取消、参数连线覆盖及图/独立调用一致。前端检查参数撤销和不改画布的独立调用，真实 MCP HTTP 验证大数精度。

检查通过：核心 409 项（3 项既有耗时测试忽略）、应用/MCP 34 项（1 项显式真实模型测试忽略）、前端 83 项；严格 Clippy、生产前端构建、UTF-8、520 项目录一致性和修改 Rust 文件格式通过。前 155 项人工记录不变。维护命令：`node scripts/capture-upstream-big-integer-vectors.mjs`；常规验收：`cargo test -p misclab-core --test migration_big_integer --locked`。

## 第 20 批：程序员计算器与位模式接续（2026-10-27）

上游 progCalc 映射为 `programmer_calc`，归进制与数值 / 位与字节。纯 Rust 手写词法和表达式解析，涵盖所有上游算术/位运算、括号、rotl/rotr、优先级和回绕规则。内联字宽 1–512 及有符号开关，输出报告、选定/有符号/无符号十进制文本、大端字节和视图 JSON；大整数始终不经浮点，可继续连接数论与字节分析节点。

修复无符号模式首行仍显示负数的上游问题。保留 >> 算术右移、>>> 逻辑右移的固定含义及 signed 对除法/余数/计数的影响。表达式最多 1024 字符、32 层括号/函数、128 层运算递归，禁止任意代码执行，模幂与解析中可取消。

867 条上游对照覆盖全部语法、10 种字宽和有/无符号；另验 1–512 所有字宽零/全一位模式、320 个原生 u64 对照、最长平表达式、深度/资源/错误输入、模幂中途取消、精确文本及字节连线、图与独立调用一致。前端参数撤销、结果接续和独立调用，以及真实 MCP 64 位无符号结果通过。

检查通过：核心 415 项（3 项既有耗时测试忽略）、应用/MCP 34 项（1 项显式真实模型测试忽略）、前端 84 项；严格 Clippy、生产前端构建、UTF-8、521 项目录一致性和修改 Rust 文件格式检查通过。前 156 项验收记录不变。维护命令：`node scripts/capture-upstream-programmer-vectors.mjs`；常规验收：`cargo test -p misclab-core --test migration_programmer --locked`。
