# 设计决定记录

规范没写清、或者自己做了选择的地方，都记在这里。按步骤分节；每条写"选了什么、为什么"。

## 第 1 层 · 1a 解析

### 总体

- **规范版本**：`docs/spec/` 里的规范是 2026-10-15 下载的最新版。HTML 规范这一年有几处大改，实现按这一版，不按旧版：分词器加了处理指令（`<?target data?>`，13.2.5.72–76），`select` 不再有独立的插入模式（按 `in body` 处理，`select` 是作用域边界），新增 `search`、`dialog`、`rb`/`rtc` 等。
- **没有第三方库，也没有开发依赖**。测试需要的 JSON 读取自己写了一个很小的（`tests/common/mod.rs`）。
- **生成的数据文件**都由 `tools/` 下的脚本从 `docs/spec/` 生成，生成物提交进仓库：`gen_entities.py`（字符引用表，`entities.bin`）、`gen_labels.py`（编码标签）、`gen_quirks.py`（怪异模式的 DOCTYPE 表）、`gen_foreign.py`（SVG 标签/属性大小写表）、`gen_colors.py`（148 个颜色名）。`prepare_zhwiki.py` 准备性能测试页面（不提交）。
- **没有报告解析错误**（HTML、CSS、URL 都不收集 validation error）。第 1 层用不到；测试里只比对结果，不比对错误列表。
- **规范里的 "parse error" 一律按规范规定的恢复方式处理**，不报错也不终止。

### 网址（`url.rs`）

- 按 WHATWG URL 4.4 的状态机逐状态实现；`special scheme`、Windows 盘符特例、`file:` 主机、不透明路径、`/.` 前缀都做了。
- **IDNA 简化**（任务单已定）：非 ASCII 域名只做：Unicode 小写、全角 ASCII（U+FF01–FF5E）折成半角、`。` `．` `｡` 当 `.`、忽略 U+00AD/200B/2060/FEFF、U+00A0/U+3000 变成空格（于是失败）、U+FFFD 和非字符失败，然后每个含非 ASCII 的标签做 Punycode。不做 NFC/NFKC 和 UTS #46 完整映射。
  - WPT `urltestdata.json`：896 条通过 895 条。唯一失败：`file://loC𝐀𝐋𝐇𝐨𝐬𝐭/usr/bin`（数学粗体字母要 NFKC 折叠成 `localhost` 才对）。
- 查询串编码目前固定 UTF-8（标准里非 UTF-8 页面的表单提交才会用别的编码），1b 通过平台接口补。
- `origin()` 按 URL 4.7 实现了 tuple origin（http/https/ws/wss/ftp 和 `blob:` 里的 http(s) 网址），其余是 `"null"`；1c 的 Referer 策略要用。

### 编码（`encoding/`）

- 标签表（`labels.rs`）由 Encoding 标准 4.2 的表生成（228 个标签、40 个编码）。引擎内部直接解 UTF-8、UTF-16LE/BE、**windows-1252**、`x-user-defined`、`replacement`；其余编码调用 `Platform::decode_legacy(标准编码名, 字节)`。windows-1252 在引擎里自己解，是因为"平台没有这个编码时退到 windows-1252"必须不依赖平台；任务单里 `iso-8859-1`/`us-ascii` → 代码页 1252 也因此自然成立。
- 平台钩子收到的是 **WHATWG 编码名**（`"GBK"`、`"Big5"`、`"Shift_JIS"`、`"EUC-KR"`、`"gb18030"` …），不是代码页号。名字到代码页的对照（`gb2312`/`gbk`/`gb18030` → 54936，`Big5` → 950，`Shift_JIS` → 932，`EUC-KR` → 949 …）放在 `qinglan-win`（1b），引擎保持不认识 Windows。
- 判断顺序：BOM → HTTP `charset` → `<meta>` 预扫描（HTML 13.2.3.2，前 1024 字节，含 `http-equiv`/`content` 写法、UTF-16 声明改当 UTF-8、`x-user-defined` 改当 windows-1252、`<?xml ... encoding>`）→ 前 4096 字节是合法 UTF-8 就用 UTF-8，否则 GB18030。**被 4096 字节边界截断的最后一个字符不算错误。** 有任何声明就不再猜。
- 平台不认识的编码：按 windows-1252 解，`Decoded.warning` 里写明。
- 样式表解码（`decode_stylesheet`）：BOM → HTTP charset → 文件开头的 `@charset "…";` → 链接它的页面的编码。

### HTML 分词与建树（`html/`）

- **html5lib 分词测试**：7045 个用例（含每个初始状态），全部通过；另有 4 个用例**跳过**：`unicodeCharsProblematic.test` 里需要孤立代理项（U+D800、U+DFFF）的 4 条——Rust 的 `str` 装不下孤立代理项，真实输入走 UTF-8 解码也不会出现。`xmlViolation.test`（XML 信息集强制转换）不适用，没跑。
- **html5lib 建树测试**：1784 个脚本关闭可跑的用例（另有 8 个 `#script-on` 的跳过），**1776 个通过（99.55%）**，8 个没过：
  - 4 条是**测试数据比规范旧**：`html5test-com.dat#12`、`tests1.dat#40/#44/#47`。规范现在把 `<?` 开头的内容解析成处理指令（`<?COMMENT?>` 是目标为 `COMMENT` 的处理指令，不再是注释；`<?` 后面直接结束的输入什么都不产生）。html5lib-tests 的建树测试已在 2026-06-26 搬到 WPT，我们能下载到的最后一份还是旧语义（见 `tests/html5lib/SOURCE.txt`）。分词测试用的是 2026-09-29 的最新版，那一版已经含处理指令的用例，全部通过。
  - 4 条是 `<selectedcontent>`（`webkit02.dat#45–48`）：期望把选中 `<option>` 的内容复制进 `<selectedcontent>`。那是 DOM 层的步骤（在表单控件一章，不在解析一章），解析规范里没有，没做。
  - 这 8 条写在 `tests/html5lib_tree.rs` 的 `KNOWN_FAILURES` 里，出现任何别的失败测试都会失败。
- **处理指令**：令牌、DOM 节点（`NodeData::ProcessingInstruction`）、各插入模式都按规范做了；DOM 打印格式（`dom` 命令、测试）选 `<?target data>`。
- **脚本开关为"关"**：`<noscript>` 当普通 HTML 解析；不会运行脚本，`<script>` 的内容当脚本数据保留在 DOM 里。
- **没做**：声明式 Shadow DOM（`<template shadowrootmode>` 当普通 `template`）、`template for` 内容补丁、解析器清洗配置（sanitizer）、`document.write` 重入、推测解析。规范里这些步骤都依赖脚本或 DOM API。
- **文本令牌是一段一段给的**，不是一个字符一个令牌。树构造器把每段按"开头的空白 / NUL / 其余"切成同类的几块再处理，这和逐字符处理等价，唯一例外是 `frameset` 相关三个模式（留空白、丢文字），那里按空白/非空白交替切得更细。`<pre>`、`<textarea>`、`<listing>` 后面紧跟的第一个换行在令牌层面丢弃。
- **深度上限 512**：开放元素栈到 512 层后，新元素挂在第 512 层那个元素下面，**而且不入栈**（所以之后它的结束标签是找不到匹配的，会被忽略）。极端坏页面里这会让闭合的位置和浏览器不同，但深度有界、每个标签的处理量有界（作用域检查最多走 512 层）。空元素（`br`、`img` …）从不入栈。
- **节点上限 50 万**：每处理完一个令牌检查一次，超过就停止解析，`Document::too_large = true`（页面顶部的提示由 1b/1c 画）。实际节点数可能比 50 万略多（一个令牌最多创建几个节点）。
- **每个标签最多保留 2048 个属性**，多的丢掉，原因是重复属性检查是平方级的（一个元素 10 万个属性是恶意页面测试里的一项）。
- **字符引用表**存成 `entities.bin`（2231 个名字，按名字排序，25.8 KB，名字不重复存储）；没有再做前缀压缩——需要时可以再压，现在这个大小对 3 MB 的预算足够。匹配用二分逐字符收窄，取最长匹配，属性里的"后面跟 `=` 或字母数字就不解码"的历史规则照做。
- **怪异模式的 DOCTYPE 判断**表由规范文本生成（`html/quirks_data.rs`）。
- DOM：节点放数组、用 `u32` 当下标（`NONE = u32::MAX`）；标签名、属性名、类名、id 驻留成 `Atom`（一张表，`Interner`，静态名字的编号编译期固定）。`<template>` 的内容是一个单独的 `Fragment` 节点（`template_contents`）。DOCTYPE 装箱以减小节点。
- 片段解析（HTML 13.4）也实现了，建树测试里的 `#document-fragment` 用例要用；引擎对外还没有入口用到它（1c 之后 `innerHTML` 类的需求才有）。

### CSS（`css/`）

**范围与丢弃规则**

- 只实现任务单列出的属性；其余属性的声明**当作无效丢弃**（`@supports (opacity: 1)` 因此是假）。逻辑属性（`margin-inline-start`、`inset`、`block-size` …）按 `horizontal-tb` + `ltr` 映射到物理属性，因为第 1 层 `direction` 只解析、不改变排版方向。
- `@font-face`、`@keyframes`、`@container`、`@page`、`@namespace`、`@property`、`@scope` 等整条忽略。CSS 嵌套不支持（块里的嵌套规则被丢弃，后面的声明照常）。选择器里的命名空间前缀：`*|name` 当 `name`，`|name` 永不匹配，`prefix|name`（没有 `@namespace` 声明）无效。
- `@import` 必须在所有规则前面（只有 `@charset`、`@layer` 声明语句可以在前），否则忽略；支持 `layer`、`layer(名)`、`supports(…)` 和媒体查询。每页最多 64 个样式表（`<style>`、`<link>`、`@import` 合计），`@import` 最深 4 层，链上出现过的网址不再加载（防循环）；一个页面最多 30 万条规则。
- `<link>` 只认 `rel` 含 `stylesheet` 且不含 `alternate` 的；`type` 不是空或 `text/css` 的不加载；`disabled` 的不加载；`media` 按当前视口求值。

**值**

- `ex`、`ch`：1a 时按 0.5em；1b-1 起取平台的字体度量（x 高度、"0" 的前进宽度），没有平台时仍是 0.5em，见 1b-1 一节。`vw/vh/vmin/vmax` 按 `style` 命令的 `--width/--height`（默认 1280×1000）；`dvh/svh/lvh` 等当 `vh`，`dvw` 等当 `vw`。
- 颜色：Lab/LCH/OKLab/OKLCH 换算到 sRGB 后**直接钳制**到 0–255，不做色域映射（规范推荐的映射第 1 层用不上）；`color-mix()`、`color()`、`light-dark()`、相对颜色语法都当无效。系统颜色（`Canvas`、`LinkText`、`ButtonFace` …）取一组固定的浅色方案的值（`css/color.rs`）。`currentcolor` 在边框、背景、装饰线颜色上保留为关键字，在 `color` 上等于 `inherit`。
- `calc()`/`min()`/`max()`/`clamp()` 嵌套最多 32 层；加减号两侧必须有空白；类型不匹配（长度 + 数字）无效；算出来不含百分比就在计算值阶段收成一个长度。没有 `round()`、`mod()`、`sin()` 等。
- `font-size` 绝对关键字用 Blink 的表（16px 为 `medium`）；**只写 `monospace` 一个字体族时 `medium` 是 13px**（Blink/Gecko 的老规矩，不然 `<pre>`、`<code>` 会比浏览器大）。继承来的关键字大小在字体族变成/不再是 `monospace` 时会重新算（`size_keyword`）。`larger/smaller` 按 1.2 倍。`font-weight: bolder/lighter` 按规范表。
- `position: sticky` 的计算值直接是 `relative`（任务单）；绝对定位和固定定位的元素 `float` 计算为 `none`，浮动元素和根元素的 `display` 按 CSS2 9.7 块化（`inline` → `block` 等）；`overflow` 一轴不是 `visible`/`clip` 时另一轴的 `visible` → `auto`、`clip` → `hidden`。
- `display` 的计算值**保留** `table*`、`flex`、`grid`、`ruby` 等（退路在 1b 的盒子生成里做）。`display: -webkit-box` 当 `flex`。
- 怪异模式：不带单位的数字在任务单列出的那些属性里当 px（CSS Quirks 3）；UA 表加两条（`form` 底外边距、`table` 不继承字体设置，规范 15.3.3/15.3.8）。**规范里怪异模式的 `li { list-style-position: inside }` 没采用**——浏览器并不这样做，照抄会让所有旧页面的列表标记位置不对（待用户/审查确认）。

**选择器**

- 支持的伪类见任务单；另外有：`:link`/`:any-link`（只 `a`、`area` 带 `href`）、`:root`、`:scope`（当 `:root`）、`:defined`、`:dir()`。`:hover`、`:active`、`:focus`、`:focus-visible`、`:focus-within`、`:visited`、`:target` 以及一批表单状态/媒体状态伪类（`:required`、`:read-only`、`:placeholder-shown`、`:valid` …）**解析有效但永远不匹配**。`:has()` 无效，整条规则作废（`:is()`/`:where()` 里面的宽容列表只丢掉那一项）。不认识的伪类、`::-webkit-…` 之类带前缀的伪元素无效，规则作废。
- `::before`、`::after`、`::marker`、`::first-line`、`::first-letter`、`::selection`、`::placeholder`、`::backdrop`、`::file-selector-button`（以及单冒号写法的前四个）解析有效，选择器带伪元素的规则不参与元素本身的样式。
- 属性选择器：HTML 元素的属性名按小写比；值默认区分大小写，HTML 规范里列出的那些属性（`type`、`lang`、`rel`、`align` …）不区分；`i`/`s` 标志照做。怪异模式下类名和 id 不区分大小写（解析时和匹配时都转小写）。
- 一次匹配的步数上限 2 万（超了当不匹配），一个选择器最多 128 个复合选择器（超了丢掉这个选择器），防止恶意选择器。
- 匹配速度：规则按 id → 类 → 标签（含 `:link` → `a`/`area`、`:is(标签…)`）→ 属性 → 根元素 → 通用 分桶；祖先用计数布隆过滤器快速排除，`X Y`（`X` 是单个 id/类/标签）直接用精确计数判定；与兄弟/子节点无关的规则，按"祖先上下文 + 元素自身签名"缓存匹配结果；与兄弟/子节点有关的规则（兄弟组合符、`:first-child`、`:nth-*`、`:empty`、`:disabled` …）每个元素单独匹配。最后相同的声明序列 + 相同的父样式 + 相同的 `lang` 直接共享整个计算样式（`Rc`）。

**层叠**

- 顺序（从弱到强）：浏览器默认样式 → 表现属性 → 作者样式（无 `@layer` 的最强，层按声明先后，后面的强；嵌套层里子层弱于本层）→ `style=""` → 作者 `!important`（层的顺序反过来，层内最强；无层的最弱）→ `style=""` 的 `!important` → 默认样式 `!important`。同一层同一来源内：优先级 → 出现顺序。
- **表现属性（`bgcolor`、`align` …）按作者层最弱处理**：优先级 0、排在所有作者规则之前（也在 `@layer` 里的规则之前）。`<body link>` 只对 `a`/`area` 带 `href` 生效（`vlink`/`alink` 对应的 `:visited`/`:active` 永远不匹配，所以没有效果）。`<table border>` 的单元格边框（1px inset）也是表现属性，只在 `border` 解析后不为 0（或解析出错）时生效（15.3.8）。
- `revert` 回到浏览器默认样式的那个值；`revert-layer` 回到比当前层弱的最后一个值。
- 层序号：先出现的在前，同一父层下的子层按出现顺序，层自己的规则排在它的子层之后（所以子层 < 本层 < 后一个同级层）。
- 自定义属性与 `var()`：按元素解析（`--a: var(--b)` 先解 `--b`）；循环里的**所有**属性都无效（不管有没有后备值）；`var()` 缺值又没有后备值时，使用它的声明"计算时无效"，当 `unset`；展开后的值估算大小超过 64 KB 也算无效；`var()`/后备值嵌套最多 32 层；同一元素内依赖链最多 64 层。声明里含 `var()` 的简写（`margin: var(--m)`）在计算样式时整体展开再解析，按（声明, 自定义属性表）缓存结果。`env()` 当没有定义。
- 媒体查询：`screen`/`all` 匹配，`print` 和其它旧媒体类型不匹配；认识的特征见 `css/media.rs`（`width`/`height` 含范围写法、`orientation`、`resolution`、`aspect-ratio`、`prefers-color-scheme: light`、`prefers-reduced-motion: no-preference`、`hover: hover`、`pointer: fine`、`scripting: none` …）；不认识的特征是"未知"，当假。`@supports` 在解析时就求值，被判假的块整个不进入样式表；`selector()` 用我们自己的选择器解析器判断，`font-tech()` 等一律假。
- 计算样式分五组（盒子、边框、字体、文字、杂项）加一张自定义属性表，每组 `Rc` 共享；没有声明过的继承组直接用父元素的 `Rc`，没有声明过的非继承组共用一份初始值。`lang`（最近祖先的 `lang` 属性）放在文字组里随继承传递，`:lang()` 另外沿 DOM 向上找。

**默认样式表**（`css/ua.css`、`css/ua_quirks.css`）

- 照 HTML 15.3–15.5 的 CSS 改写：逻辑属性换成物理属性；去掉第 1 层没有的东西（`unicode-bidi`、`::before` 内容、计数器、popover、appearance）；`body { margin: 8px }`（15.3.2 的默认值）；`<details>` 一律展开（不隐藏内容）；表单控件的默认外观（边框、内边距、背景、字体 13.33px）参照 Chrome 自拟，因为规范把它们交给"原生外观"。`noscript` 不隐藏（脚本关）。

### 性能（1a 验收第 5 项）

- 测试页：`tests/corpus/local/zhwiki/`，中文维基"中华人民共和国"条目 2.4 MB（51020 个节点、24963 个元素），连同它的 3 个外链样式表（215 KB + 3 KB + 12 KB）和 17 个内嵌 `<style>`（共 26 个样式表、1801 条作者规则）。准备方法：`tools/prepare_zhwiki.py`。
- 量法：`cargo test --release -p qinglan-engine --test perf -- --nocapture`（进程内跑 10 次取最快的一次，含解码、解析、读样式表、算样式）。结果见提交时的报告；这台电脑的耗时随机波动很大（同一份程序前后两次相差 20–30%），数字只在插电时有意义。
- 做过的优化：分词器里的整段扫描、属性类型直接复用 DOM 的 `Attribute`、选择器分桶 + 精确祖先计数 + 匹配缓存、样式共享、`FxHasher` 按 8 字节处理、`var()` 结果缓存。

### 限制一览（防坏网页）

| 项 | 上限 | 超了怎么办 |
|---|---|---|
| DOM 节点 | 50 万 | 停止解析，`too_large` |
| 开放元素深度 | 512 | 新元素挂在第 512 层，不入栈 |
| 每个标签的属性 | 2048 | 丢掉多的 |
| 样式表（含 `@import`） | 64 个 | 后面的不加载 |
| `@import` 深度 | 4 | 不加载 |
| 规则总数 | 30 万 | 后面的丢掉 |
| `@media`/`@supports`/`@layer` 嵌套 | 32 | 整块丢掉 |
| 组件值（括号/函数）嵌套 | 512 | 展平 |
| `calc()` 嵌套 | 32 | 无效 |
| `calc()` 一个乘除链的因子数 | 32 | 无效 |
| 选择器复合数 | 128 | 丢掉这个选择器 |
| 选择器列表嵌套（`:is`/`:where`/`:not`/`:nth-child(… of S)`） | 32 | 选择器无效 |
| 函数伪类里的复合数 | 16 | 选择器无效 |
| `@media`/`@supports` 条件嵌套（含 `selector()`） | 32 | 媒体查询无效（`not all`）/ 条件为假 |
| 一次选择器匹配 | 2 万步 | 当不匹配 |
| `var()` 展开后的值 | 64 KB | 计算时无效 |
| 长度绝对值 | 1e9 px | 钳制 |
| 匹配缓存 | 20 万条 | 之后不再缓存 |

### 1a 审查后的补充（2026-10-15）

- 嵌套上限（防栈溢出）：原先只有组件值嵌套 512，对递归的解析和求值太深（debug 构建 200 层 `:is()` 就崩）。现在另设：选择器列表嵌套 32 层（`:not` 超限整条规则丢掉；`:is`/`:where` 是宽容列表，只丢掉那个选择器，空的 `:is()` 合法但匹配不到任何元素）；函数伪类里的复合选择器最多 16 个（匹配时每个复合递归一次，嵌套列表用小上限，顶层仍是 128）；`@media`、`@supports` 条件（含 `selector()`）括号/函数嵌套 32 层，超限则媒体查询无效、`@supports` 为假（`@import ... supports()` 同理）；`calc()` 一个乘除链最多 32 个因子（乘除链是左深的树，求值和释放都递归）。512 的组件值上限不变，它是兜底。`@media`/`@supports`/`@layer` 块嵌套 32、`calc()` 嵌套 32、`var()` 引用链 64 原来就有。压力测试（`tests/robustness.rs`）在 1 MB 栈的线程上跑 1 万层的各种构造，通过则 CLI 的 debug 构建（Windows 主线程 1 MB）也不会溢出。
- 嵌套 `<template>`：EOF 时 "in template" 模式要反复"弹出到 template、重置插入模式、重新处理 EOF"（HTML 13.2.6.4.18），原来用递归，每层 template 一个栈帧，debug 构建 600 层就溢出；改成循环，步骤不变。
- 旧式颜色（HTML 2.3.5 第 6 步）：大于 U+FFFF 的字符换成两个 `0`（不是一个），之后才按 128 个字符截断。`bgcolor="1😀2"` 得 `#100200`。
- 1a 性能目标（zh.wikipedia 2.4 MB 页面，解析+样式 ≤ 80 ms）没有达到，实测 84–95 ms。暂时接受：真正的关口是 1c 的首屏 ≤ 200 ms；如果它有风险，第一个优化是相同样式的元素之间共享样式（style sharing）。
- `ex`、`ch` 在 1b 之前按 0.5em 算；1b 起用字体度量（x 高度、"0" 的宽度），1b-1 已改（见下）。
- 怪异模式的 `li { list-style-position: inside }` 那条规则仍然不采用，和浏览器一致。

## 第 1 层 · 1b-1 平台接口、盒子树、块排版、基础行内排版

代码在 `platform/`、`layout/`（`boxtree.rs`、`block.rs`、`inline.rs`、`units.rs`、`dump.rs`）、`textclass.rs`；测试在 `tests/layout_snapshots.rs`（快照）、`tests/layout_basic.rs`，网页在仓库根的 `tests/pages/`。

### 平台接口（`platform/mod.rs`，一次定全）

- 方法：`decode_legacy`（1a 已有）、`resolve_font`（字体族列表 + 粗细 + 斜体 + 语言 → `FontId`）、`font_metrics`（上升、下降、行距、x 高、"0" 前进宽度、下划线和删除线的位置和粗细，单位 px）、`shape`（一段同样式的文字 → 字形列表，含回退，每个字形带字体、字形号、前进宽度、偏移、对应的字节范围 `cluster_start..cluster_end`）、`line_breaks`（整段文字的断行位置）、`glyph_mask`（灰度遮罩，横向 4 档）。对象安全，不含 Windows 类型。没有字体的 `BasicPlatform` 只是让 1a 的代码继续能用。
- **断行分工**：平台对整个段落调一次，只返回 UAX #14 的基础断点（字节位置）。`word-break: break-all / keep-all`、`white-space`（`nowrap`、`pre`、`break-spaces` 的"每个空格后可断"）由引擎叠加，`overflow-wrap` / `word-break: break-word` 在填行时处理（行里没有别的断点才在字里断）。1b-3 的 Windows 平台因此只需要把 `AnalyzeLineBreakpoints` 的结果翻译成字节位置。
- 整形的结果按"字形簇"切成最小单位，一行只在簇的边界断开；一个簇多个字符（连字）或多个字形（附加符号）都能处理。字体回退发生在平台里，`Glyph.font` 可以不同。字形号 0 表示缺字（1b-3 验收"没有方块字"就数它）。
- 尺寸一律 `f32` px，进了排版才换成 1/64 px 的整数。

### 测试平台（`platform/test.rs`）

- 按任务单：每个字形 1em 见方（前进宽度 = 字号），上升 0.8em，下降 0.2em，行距 0。**x 高度 0.8em**（Ahem 的 OS/2 表里就是这个值，"x" 字形和上升一样高），**"0" 的前进宽度 1em**；所以 `5ex` 在 20px 字号下是 80px，`2ch` 是 40px。零宽：ZWSP/ZWNJ/ZWJ/词连接符/BOM/变体选择符/组合记号/控制字符。空格不画（遮罩为空）。
- 断行（比任务单多写了几条，都是为了混排不出怪样子）：空格（和制表符等）之后可断；两个中日韩字符之间可断；中日韩字符和非中日韩字符之间可断；ZWSP 之后可断。**后置标点前不断**：`，。、；：？！）」』】》…` 等全角的，加上 ASCII 的 `, . ; : ! ? ) ] }`（UAX #14 的 CL/EX/IS 类，不加的话"界,"会在逗号前断开）；**前置标点后不断**：`（「『【《` 和 `( [ {`。连字符、不间断空格（U+00A0）、词内不断。韩文音节也算"中日韩"（UAX #14 里音节之间默认可断）。字符表在 `textclass.rs`，手写，只覆盖中日韩常用区。

### `ex` / `ch`（css-values-4 6.1）

- 样式计算时用平台度量：`ex` = x 高度，`ch` = "0" 的前进宽度，按元素自己的字体（`font-size` 里的 `ex`/`ch` 用父元素的字体）。度量查询按（字体族、粗细、斜体、语言）缓存。字体没有 x 高度或没有 "0" 时按 0.5em。`style_document_with(.., Some(platform))` 才查平台；`style_document` 和不带 `--test-font` 的 `style` 命令仍是 0.5em。

### 盒子树（`layout/boxtree.rs`，任务单第 9 节）

- 按 `display` 生成：`none` 不生成；`contents` 本身不生成、子节点顶替它的位置；`block / list-item / flow-root` 是块；`inline` 是行内盒。SVG / MathML 元素不生成（第 1 层不显示）。
- **退路**：`flex`、`grid`、`table` 和所有 `table-*`（含 `table-column`、`table-column-group`，不按 CSS2 17.2 当 `none`，照任务单"当 block"）→ 块，盒子上记着原来的 `display`，转储里写 `[display:flex]`；`inline-block`、`inline-flex`、`inline-grid`、`inline-table` → **1b-1 暂时也当块排**（等 1b-2 做了真正的原子行内盒再改），转储里同样有标记；`ruby` / `ruby-text` → 行内；`float` 忽略（1a 的层叠已按 CSS2 9.7 把浮动元素块化，所以浮动的 `span` 是个块）；`position: absolute / fixed` 也先留在正常流里（定位是 1b-2）。
- 匿名块（CSS2 9.2.1.1）：块容器的子盒里既有块级又有行内级时，每一段连续的行内级盒包进一个匿名块（继承属性来自父盒，其余用初始值）。行内元素里有块级后代时，行内元素被拆成几段（`SplitPart` 记着第一段、最后一段，给 1b-2 的左右边框用）；没有子盒的段不生成（1b-2 里有水平内边距/边框/外边距的要保留，`finish_inline` 里有注）。
- 只有可折叠空白的文字、和空的行内元素，在块之间不生成匿名块（CSS2 9.2.2.1）；`white-space: pre-line` 里含换行的空白文字要保留（它会渲染成一个空行）。
- **层数上限 `MAX_BOX_DEPTH` = 256**：嵌套更深的元素不再生成自己的盒（当 `display: contents`），子孙上移。防排版递归爆栈；测试 `deep_nesting_fits_in_one_megabyte_of_stack` 在 1 MB 栈的线程上跑 1500 层。

### 块排版（`layout/block.rs`，任务单第 10 节）

- 单位 `LayoutUnit` = 1/64 px 的 `i32`，加减乘都饱和在 ±2^24 px，1e9px 的恶意长度不会溢出。样式里的 `f32` 长度四舍五入换算；文字宽度按每个字形簇四舍五入到 1/64（测试字体里全是整数，不受影响）。
- 一个块只需要知道含块宽度（和已定的高度）就能排；它自己的位置由父亲事后定。外边距合并（CSS2 8.3.1）自下向上算：每个块报告上、下边缘露出来的外边距（已经和第一个/最后一个子块合并）和"自己的上下外边距穿过自己"（`through`）；父亲在兄弟之间合并并摆放。覆盖：兄弟、父子（上、下）、空块穿过、负外边距（最大正值加最小负值）、padding / border 隔断、新格式化上下文（`overflow` 非 visible/clip、`flow-root`、各种退路块、根元素）。
- 空块（`through`）的位置按 8.3.1 末段："像下边框不为零那样"——只算它自己的上外边距，所以 `g1 { margin: 20px 0 30px }` 夹在两块之间时，它在上一块下面 20px 处，而后一块在 30px 处。
- 宽度 10.3.3：auto 外边距、过约束（ltr：右外边距被忽略）、两个外边距都是 auto 时居中，装不下时左 0、右取余；min/max 10.4（算一遍，超过 max 用 max 再算，小于 min 用 min 再算）；`box-sizing: border-box` 的 width / min / max 先减去 padding + border（不小于 0）。`min-content / max-content / fit-content` 1b-1 当 `auto`（要内在宽度，1b-2 才有）。
- 高度 10.6.3、10.5：百分比高度只有含块高度明确（含块 `height` 指定了，或是根的视口）才算，否则当 `auto`；`min-height` 的百分比在含块高度不明确时当 0，`max-height` 当 `none`。边框宽度和 Blink 一样取整（≥ 1 取整，0–1 之间算 1px）。padding 的百分比也是对含块**宽度**。
- **`overflow`**：`hidden` 和 `clip` 记 `[clip]`（1c 绘制时裁剪）；`auto` / `scroll` 按任务单当 `visible`，内容不丢，不裁剪。但 `hidden / auto / scroll` 仍然**是格式化上下文**（和浏览器一致），子块的外边距不会合并出来；`clip` 不是。
- 根元素是格式化上下文：它的外边距不和子元素合并。视口高度只用来解析根上的百分比高度（`html { height: 100% }`）。

### 行内排版（`layout/inline.rs`，任务单第 10 节）

- 一个"子盒全是行内级"的块容器是一个段落：先收集文字（`collect`）、按 css-text-3 4.1.1 / 4.1.3 做空白折叠（**第一阶段**，跨行内元素边界），再整形、切单位、断行、排行（**第二阶段**，4.1.2：行首行尾的可折叠空格删掉，保留的空格在行尾"悬挂"，`pre-wrap` 在强制换行前的悬挂空格算有条件悬挂，放得下就不悬挂）。
- 六种 `white-space`（`white-space-collapse` × `text-wrap-mode`）：`normal`、`nowrap`、`pre`、`pre-wrap`、`pre-line`、`break-spaces` 的行为见 `tests/pages/white-space.html`。`preserve-spaces`（css-text-4）：空格保留，换行变成保留的空格。`\r` 当空格。制表符：折叠模式变空格；保留模式按 `tab-size: 8`（`tab-size` 属性 1a 没解析，用初始值）走到下一个制表位（制表位从块的内容左边算，不足半个空格宽就跳到下一个）。
- **中日韩换行（4.1.3，Firefox 的做法，Chrome 没有）**：可折叠的换行，两边的字符的东亚宽度都是 W / F / H 且都不是韩文，就删掉不变成空格；任何一边是 U+200B 也删掉；其余变空格。先删换行两边的空格和制表符，连续的换行算一个。判断跨文字节点、跨行内元素。韩文要空格（"안녕\n하세요" → "안녕 하세요"）。
- **断点叠加**：见上"断行分工"。两个相邻单位属于不同文字项时，"任一边允许换行就算允许"（真正的规则看共同祖先的 `white-space`，这是近似）。
- **行高**：`line-height: normal` = 主字体的上升 + 下降 + 行距，三个数先取整到整 px（和 Blink 一样）；`<数字>` 乘元素自己的字号。半行距 = (行高 − 上升 − 下降) / 2，**上半部分向下取整到整 px**（Blink 的做法，CSS 没规定怎么舍入），下半部分是余数。行盒的高度 = 支柱和这一行里有内容的每个行内盒（包括文字外面的所有行内祖先、`<br>` 自己）的（上升 + 上半行距）最大值 + （下降 + 下半行距）最大值；没有内容的空行内盒不参与（和 Chrome 一致）；不实现怪异模式里"没有文字的行不用支柱"的老规矩。
- 只有强制换行（`<br>`、保留的换行符）结尾的空行有支柱的高度；末尾的 `<br>` 不再生出一个空行。
- `text-align`：`left` / `start` / `match-parent` / `justify` / `justify-all` → 靠左（`justify` 要到 1b-2），`right` / `end` → 靠右，`center`。内容比行宽时不居中也不靠右，从行首开始（css-text-3 7.1）。没有右到左排版，`direction` 不影响。居中的偏移 = 空余 / 2（1/64 单位向零取整）。
- 文字片段（转储里的 `text`）的矩形是内容区：高 = 主字体上升 + 下降，底边不含半行距；同一行里同一个文字项的相邻单位合成一个片段。
- 没做（1b-2）：`vertical-align`（只有 baseline）、行内盒的 padding / border / margin、`justify`、`text-indent`、`letter-spacing`、`word-spacing`、`text-transform`、装饰线、原子行内（`inline-block` 暂时按块）、图片、列表标记、定位、表单控件、**排版工作量计量**（现在只有 `MAX_BOX_DEPTH`；一个 10MB 没有空格的文字现在会整个整形，每个字符约占 100 字节）。

### `qinglan-cli layout` 和转储格式

- `qinglan-cli layout <文件> [--width 1280] [--height 1000] [--test-font] [--time]`。不带 `--test-font` 报错退出（"Windows 平台到位前只有 --test-font 能用"）。
- 转储格式（`layout/dump.rs` 开头有例子）：第一行 `viewport WxH`；之后每个块一行 `block <标签#id.类 | (anonymous)> (x,y WxH)`，坐标是页面坐标下的边框盒，单位 CSS px，去掉多余的 0，最小单位 1/64 = 0.015625；退路块后面加 `[display:flex]` 之类，`overflow: hidden/clip` 加 `[clip]`。块里有行内内容时，下面缩进列出 `line (x,y WxH) baseline=Y`（Y 是基线的页面 y）和它的 `text "…" (x,y WxH)` 片段，强制换行是 `br`（`<br>` 元素）或 `newline`（保留的换行符）。
- 快照：`tests/pages/*.html` 配 `*.layout.txt`。每个网页开头的 HTML 注释里写着按规范**手算**的关键数字，其中 `check: <行首文字> :: <期望片段>` 的行由测试程序逐条对照转储（手算数字不是从引擎抄来的，所以快照不会只是记录引擎当时的输出）；`viewport: WxH` 设视口。重新生成期望文件：`UPDATE_SNAPSHOTS=1 cargo test -p qinglan-engine --test layout_snapshots`，生成前先确认手算数字没有失败。

### 来源

- html5lib-tests：分词 `c777c408`（2026-09-29），建树 `9329e646`（2026-06-20，上游已搬到 WPT），MIT，见 `tests/html5lib/LICENSE`、`SOURCE.txt`。
- WPT `url/resources/urltestdata.json`，BSD-3-Clause，见 `tests/wpt-url/LICENSE.md`。
- 规范文本放在 `docs/spec/`（不提交），2026-10-15 下载。
