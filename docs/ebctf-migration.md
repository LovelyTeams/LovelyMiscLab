# EBCTFCodeBox → LovelyMiscLab 迁移

**当前状态（2026-10-27）：按用户要求暂停迁移，进入桌面功能测试。已验收135/820项，后续批次等待用户恢复。**

按完整清单分批用 Rust 实现。目标仓库保留现有节点图架构，不引入上游 JS、WASM、Python 或外部工具作为新增内置算法的运行依赖。

参考版本固定为 [ea2bc36](https://github.com/Henglie/EBCTFCodeBox/tree/ea2bc36dae31c97060f7f2cddad8df5518afad7b)，不是随 `main` 漂移的目标。完整 **820 项操作**见 [逐项表格](ebctf-operations.md) 和 [机器可读 JSON](ebctf-migration.json)，每项保留 ID、名称、分类、family、来源文件、目标节点和验收状态。

| 状态 | 数量 | 含义 |
|---|---:|---|
| migrated | 135 | 已实现或核验，有参数映射、固定向量和边界测试 |
| existing-review | 49 | 已有名称匹配候选，待核验参数、变体和行为 |
| pending | 636 | 待迁入，或需确认能否由现有节点组合实现 |

`existing-review` 不计作完成。一个上游操作可能拆成多个 Rust 节点，也可能由某节点的参数模式覆盖；节点数不能直接视作上游覆盖率。首批 13 项 / 24 节点保留，第二批新增 20 项 / 36 节点，第三批核验并补齐 8 项 / 16 个既有 Base 节点，第四批 10 项新增 11 个节点并补齐 8 个既有节点。第五批 20 项新增 35 个方向和报告节点。第六批 20 项新增 32 个地址、时间和分析节点。第七批 7 项新增 14 个 Unicode Base 与补位隐写节点。第八批 5 项新增 9 个浮点、字符集和时间戳节点。第九批新增 3 个二进制结构解析节点。第十批新增 3 个对象序列化检查节点。第十一批新增 21 个校验与记数节点。第十二批新增18个字符集节点。第十三批新增10个方向节点并扩展2个既有Hexdump节点。加上 2 个产物 I/O 节点，当前注册表总计 **489** 个节点。

人工验收源是 [ebctf-acceptance.json](ebctf-acceptance.json)，记录批次、方向、参数映射、目标节点、分类、测试及兼容差异。生成器只写派生清单和文档，校验每批不超过 20 项、映射可解析、分类与默认参数未漂移；不会按名称匹配提升完成状态。当前共 13 个批次：13 / 20 / 8 / 10 / 20 / 20 / 7 / 5 / 3 / 3 / 11 / 9 / 6 项。

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
- 计划显示说明、全部节点、完整参数和连线，明确展示追加/替换选择。确认前不改变画布；修改输入或画布后需重新生成计划。
- 确认后只搭建、执行已展示的子图，使用普通 Rust 图执行器；执行期间不再让模型追加步骤。搭建整体可一次撤销，取消时保留已经搭建的节点。
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

## 累计验证记录（2026-10-27）

- Rust 核心：370 通过，3 个原有耗时 bkcrack 测试按默认配置忽略。
- Tauri 应用（包含 MCP）：20 通过，包含分类层级/计数、旧分类查询、上游别名和 AI 目录签名。
- 前端工作流：26 通过，包含节点级/任务级错误、任务启动前取消、旧结果丢弃、类型化输入、独立调用锁和示例填入。
- `cargo clippy --all-targets -- -D warnings`、`pnpm build`、`pnpm check:utf8`、Git 差异空白检查通过。
- 第四批浏览器验证：344 个模块按 50 项分页，分类与二级分组正确；上游 ID 可搜索 multilineBase64、mixHexOctBin；独立调用示例可编辑，奇数位 Hex 在执行前报错；新增混排节点可加入画布、内联参数同步侧栏，两次撤销恢复测试前四节点流程。先前批量删除/一次撤销、Base36 添加、AI 计划入口验证保持有效。
- 所有本次改动的 Rust 文件已格式化并逐文件检查。迁移开始时记录了 164 个原有格式差异文件，本次仅格式化涉及的文件，未把其余文件整体重新排版混入迁移。
- 未使用真实 AI 端点进行联网端到端测试；计划确认与执行隔离使用模拟传输回归测试，Rust 后端计划解析使用离线测试。
