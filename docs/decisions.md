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

- `ex`、`ch` = 0.5em（引擎核心没有字体度量；1b 如果要更准再改）。`vw/vh/vmin/vmax` 按 `style` 命令的 `--width/--height`（默认 1280×1000）；`dvh/svh/lvh` 等当 `vh`，`dvw` 等当 `vw`。
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
- `ex`、`ch` 在 1b 之前按 0.5em 算；1b 起必须用真实的字体度量（x 高度、"0" 的宽度）。
- 怪异模式的 `li { list-style-position: inside }` 那条规则仍然不采用，和浏览器一致。

### 来源

- html5lib-tests：分词 `c777c408`（2026-09-29），建树 `9329e646`（2026-06-20，上游已搬到 WPT），MIT，见 `tests/html5lib/LICENSE`、`SOURCE.txt`。
- WPT `url/resources/urltestdata.json`，BSD-3-Clause，见 `tests/wpt-url/LICENSE.md`。
- 规范文本放在 `docs/spec/`（不提交），2026-10-15 下载。
