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
- 量法：`cargo test --release -p qinglan-engine --test perf -- --nocapture`。进程内跑 10 次，每一段都打印中位数、最小、最大值：解码、解析、样式（目标看三者之和的**中位数**，不是最快的一次）；另外再跑 10 次带计时的，把样式拆成读样式表、解析样式表、建规则索引、`style` 属性、级联（级联里再分签名、匹配、建条目、兄弟相关规则、表现属性、共享查找、算计算值）和各种命中计数。带计时的那一遍每个元素要读 7 次时钟，比不带计时的慢一点，只用来看比例。
  测试只卡一个宽松的回归上限（release 下总耗时中位数 ≤ 150 ms）；70 ms 的目标只报告，不断言，因为它和机器有关。这台电脑耗时波动很大（同时开着鸿蒙模拟器，同一份程序前后两次可差 10–20%），所以比较前后用旧、新两个可执行文件交替跑。
- 之前做过的优化：分词器里的整段扫描、属性类型直接复用 DOM 的 `Attribute`、选择器分桶 + 精确祖先计数 + 匹配缓存、按声明列表共享样式、`FxHasher` 按 8 字节处理、`var()` 结果缓存。
- **2026-10-15 的“加快样式计算”**（目标：解码 + 解析 + 样式的中位数 ≤ 70 ms；同一时段、新旧程序交替跑，4 次的中位数）：
  | 阶段 | 之前 | 之后 |
  |---|---|---|
  | 解码 | 2.0 ms | 2.1 ms |
  | 解析（HTML） | 约 33 ms（32–39） | 约 26 ms |
  | 样式 | 约 76 ms（72–80） | 约 43 ms（42–46） |
  | **合计（中位数）** | **约 118 ms**（110–123，最快一次 93–98） | **约 74 ms**（71–76，最快一次 62–67） |
  样式内部（带计时那一遍，之前 → 之后）：读样式表 0.8 → 0.8；解析样式表 16.7 → 15.5；建规则索引 1.2 → 0.6；`style` 属性 1.7 → 1.7；级联 59.8 → 28（签名 7.1 → 6.0，无兄弟依赖规则的匹配 25.3 → 6.4，建条目 3.0 → 0.6，兄弟相关规则 5.4 → 2.5，表现属性和排序 5.2 → 1.8，共享查找 2.5 → 1.6，算计算值 4.3 → 4.1）。级联里 24963 个元素中 22884 个直接命中匹配缓存，20942 个的样式不经过建条目、排序，直接取缓存里的结果。
  **目标没有达到**：这台电脑上本次的中位数约 74 ms，比 70 ms 高；样式部分已经从约 76 降到约 43 ms，剩下的大头是 HTML 解析（26 ms）和样式表解析（15.5 ms）。
  **复测（2026-10-15，插电、关掉模拟器后）：达标。** 连续 5 次，每次 10 轮的中位数：58–60 ms（第一次 68 ms，含预热）；其中解析约 21 ms、样式约 36 ms、样式表解析约 13 ms。上面 74 ms 是开着模拟器时量的。
  做了什么（按收益大小）：
  1. **匹配缓存的签名只看选择器会看的东西**。原来签名里放了 `href`、`type` 等属性的整个值，每个链接的值都不同，所以一半的元素都没命中缓存（12159 次未命中）。现在属性只放“有没有”，加上选择器里每个属性选择器（`[href^=…]`、`[title=… i]` 等）对这个元素的**判断结果**；`type`、`lang`、`dir` 仍放整个值（`:checked`、`:lang()`、`:dir()` 要读值）。同一个属性名上的属性选择器超过 48 个时退回放整个值（防止坏页面把每个元素的工作量放大）。未命中降到 2079 次。这个做法和 Stylo 的样式共享缓存里的“revalidation”类似（把缓存没法用标签/类/id 表示的选择器，逐个求值再比较结果），但是我自己按本项目的数据结构写的，没有对照 Stylo 的源码，也没有照搬它的候选判断条件。
  2. **缓存里直接存算好的样式**。匹配缓存的每一项（祖先链签名 + 元素签名）另存最多 8 个 (父元素样式, 样式) 对；元素没有兄弟相关规则命中、没有表现属性提示、没有 `style` 属性时，只要父元素样式是同一个（`Rc::ptr_eq`），样式一定相同，直接取用，跳过建条目、排序、哈希和共享表查找。有任何一项额外输入，就走原来的路（建条目 → 按声明列表共享 → 算）。
  3. **兄弟相关规则只在可能命中时才试**。`:first-child`、`+`、`~`、`:empty` 这类规则原来对每个元素都试一遍。现在在缓存未命中时，用“把依赖兄弟/子节点的部分当作成立，其余照常比对”的宽松匹配（`Matcher::may_match_ignoring_siblings`，结果是真匹配的超集，只依赖元素本身和祖先）先筛一遍，缓存里只留候选；每个元素只对候选做真正的匹配。
  4. 表现属性提示（`HintBuilder`）按属性的 atom 查表判断“有没有相关属性”，不再对每个属性做字符串比较；`href` 只在 `<body link>` 存在时才算相关。
  5. HTML 解析：活动格式元素列表只存元素和名字，需要 token 时才从 DOM 元素上取属性（原来每个 `<a>`、`<b>`、`<i>` 都复制一份带属性的 token，约 3 万次分配；元素的属性在这之后不会变——只有 `html`、`body` 会被合并属性，它们不是格式元素）；分词器的文本和属性值不再用 `mem::take`（取走后缓冲区从 0 开始反复扩容），改成复制成刚好大小的字符串并留着缓冲区；`<body>` 里的文本整段交给树构造器，不再拆出前导空白，新文本节点直接接管这个字符串。分配次数 18.7 万 → 12.3 万。
  6. 小项：选择器在样式表和规则索引之间用 `Rc` 共享，不再复制；`gather` 去重在原地做；CSS 分词器的名字用刚好大小的字符串、token 数组预留容量；`[attr=… i]` 的大小写判断在解析选择器时算好。
  **验证**：`qinglan-cli style` 在优化前后对 `tests/pages/*.html` 和 zhwiki 页面（共 16 份，11 MB）的输出逐字节相同；另外用一份专门造的页面（兄弟选择器、`:nth-child`、`:empty`、属性选择器各种运算符和 `i`/`s` 标志、`:lang`、`:checked`、带 `cellpadding`/`border` 的表格、`style` 属性、`var()`，标准模式和怪异模式各一遍）对比，也逐字节相同。
  **没做、但量过上限的**：用一个只往前走、从不释放的垃圾分配器临时替换系统分配器（只在一次性的实验里，没有提交），同一份测试合计从约 74 ms 降到约 55 ms（解析 25 → 18，样式 44 → 35）。也就是说约 19 ms 花在 Windows 系统堆的分配/释放上（解析 12 万次、样式 16 万次分配）。想继续降，有这几条路，都需要用户拍板：(a) 减少分配次数——属性值和文本用短字符串内联、CSS token 不用 `String`（约 −2.5 ms 和 −3 ms，改动面大）；(b) 在 `qinglan-win` 的 `src/sys/` 里放一个全局分配器（要 `unsafe`，对真实浏览器有效，但测试里量到的引擎不带它）；(c) 声明块延迟解析，只解析被匹配到的规则（约 −2 ms）；(d) 并行分词各个样式表（约 −3 ms，引入线程）。试过但放弃的：缓存的祖先上下文只放“选择器在祖先一侧问过的类和 id”，未命中只少了 6%，不值得多出来的复杂度。

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
| 网格自动放置（`grid.rs` 的 `place`） | 每一步扫描、每一次占位都计入排版预算：一步 1 单位，另按每 8 个 64 位字 1 单位；预算用完就停 | 还没放的项不排版，页面标记"不完整" |
| 网格里跨多条轨道的项（`resolve_intrinsic`） | 每项 2 + 4×跨度 单位；查一遍弹性轨道另计 跨度/8 | 剩下的跨轨道项不参与轨道尺寸，页面标记"不完整" |
| 浮动放置（`floats.rs`） | 每轮 1 + 已有浮动数/4 单位（行内的 `line_band` 每次 1 + 浮动数/8） | 浮动停在当前位置，页面标记"不完整" |
| 生成内容的盒子（`::before`/`::after`/引号/`counter`） | 文档节点数在 `MAX_NODES` 下留的空间的 1/4 | 后面的不生成，页面标记"不完整" |
| 网页字体：单个字体解码后（`fonts::MAX_FONT_BYTES`；WOFF/WOFF2 声明的大小、Brotli/inflate 的输出都在分配前核对） | 8 MB | 这个字体不用，文字用下一个字体 |
| 网页字体：一页所有字体解码后合计（`MAX_PAGE_FONT_BYTES`，计入内存，换页时释放） | 16 MB | 之后到的字体不用 |
| 网页字体：字体文件的表数（`MAX_TABLES`） | 128 | 拒绝这个字体 |
| 网页字体：每页 `@font-face` 规则 / 每页取的字面 / 取字体的请求 | 256 / 32 / 128 | 多的规则丢掉；其余字面不下载；`src` 里后面的不再试 |
| 网页字体：每个字面的 `unicode-range` 范围数 / `src` 条数 | 512 / 16 | 多的丢掉 |
| 网页字体：Brotli 一个流的准备工作量（前缀码、上下文映射、逆移到前；`brotli.rs` 的 `Work`，一个单位约写一个字节） | 1M 单位 + 输入每字节 16 单位 | 这个流作废（`TooLarge`），字体不用 |
| 网页字体：彩色字形各层的像素合计 / 合成用的外框 | 16 Mpx / 1 Mpx（`dwrite.rs`） | 这个字形按单色画 |
| 注音对（`ruby_pairs`，每对约 4 个盒子） | 和生成的盒子共用同一个额度（`gen_cap`） | 后面的底字和 `rt` 当普通行内，页面标记“不完整” |
| 盒阴影的遮罩（`paint/fx.rs`，切掉中间不变的行列之后的像素数 `MAX_MASK_PX`；保留 `MAX_KEPT_PX` 每条、`MAX_KEPT_BYTES` 合计） | 4 Mpx；保留 1 Mpx / 6 MB | 这个阴影不画（遮罩的大小只和圆角、模糊半径有关，盒子再大也不会超；超的只有 `blur` 大到上千像素的） |

### 1a 审查后的补充（2026-10-15）

- 嵌套上限（防栈溢出）：原先只有组件值嵌套 512，对递归的解析和求值太深（debug 构建 200 层 `:is()` 就崩）。现在另设：选择器列表嵌套 32 层（`:not` 超限整条规则丢掉；`:is`/`:where` 是宽容列表，只丢掉那个选择器，空的 `:is()` 合法但匹配不到任何元素）；函数伪类里的复合选择器最多 16 个（匹配时每个复合递归一次，嵌套列表用小上限，顶层仍是 128）；`@media`、`@supports` 条件（含 `selector()`）括号/函数嵌套 32 层，超限则媒体查询无效、`@supports` 为假（`@import ... supports()` 同理）；`calc()` 一个乘除链最多 32 个因子（乘除链是左深的树，求值和释放都递归）。512 的组件值上限不变，它是兜底。`@media`/`@supports`/`@layer` 块嵌套 32、`calc()` 嵌套 32、`var()` 引用链 64 原来就有。压力测试（`tests/robustness.rs`）在 1 MB 栈的线程上跑 1 万层的各种构造，通过则 CLI 的 debug 构建（Windows 主线程 1 MB）也不会溢出。
- 嵌套 `<template>`：EOF 时 "in template" 模式要反复"弹出到 template、重置插入模式、重新处理 EOF"（HTML 13.2.6.4.18），原来用递归，每层 template 一个栈帧，debug 构建 600 层就溢出；改成循环，步骤不变。
- 旧式颜色（HTML 2.3.5 第 6 步）：大于 U+FFFF 的字符换成两个 `0`（不是一个），之后才按 128 个字符截断。`bgcolor="1😀2"` 得 `#100200`。
- 1a 性能目标（zh.wikipedia 2.4 MB 页面，解析+样式 ≤ 80 ms）原来没有达到（84–95 ms）。2026-10-15 做了“加快样式计算”（见上面“性能”一节），把目标改成解码 + 解析 + 样式的中位数 ≤ 70 ms；这台电脑上中位数约 74 ms（基线约 118 ms），开着模拟器时仍未到 70 ms；插电、关掉模拟器后复测中位数 58–60 ms，达标。真正的关口仍然是 1c 的首屏 ≤ 200 ms，目前有富余。
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
- **退路**：`flex`、`grid`、`table` 和所有 `table-*`（含 `table-column`、`table-column-group`，不按 CSS2 17.2 当 `none`，照任务单"当 block"）→ 块，盒子上记着原来的 `display`，转储里写 `[display:flex]`；`inline-block`、`inline-flex`、`inline-grid`、`inline-table` → 1b-1 暂时也当块排，**1b-2 起是真正的原子行内盒**（见 1b-2 一节）；`ruby` / `ruby-text` → 行内；`float` 忽略（1a 的层叠已按 CSS2 9.7 把浮动元素块化，所以浮动的 `span` 是个块）；`position: absolute / fixed` 1b-1 先留在正常流里，1b-2 起脱离正常流。
- 匿名块（CSS2 9.2.1.1）：块容器的子盒里既有块级又有行内级时，每一段连续的行内级盒包进一个匿名块（继承属性来自父盒，其余用初始值）。行内元素里有块级后代时，行内元素被拆成几段（`SplitPart` 记着第一段、最后一段，给 1b-2 的左右边框用）；没有子盒的段不生成（1b-2 里有水平内边距/边框/外边距的要保留，`finish_inline` 里有注）。
- 只有可折叠空白的文字、和空的行内元素，在块之间不生成匿名块（CSS2 9.2.2.1）；`white-space: pre-line` 里含换行的空白文字要保留（它会渲染成一个空行）。
- **层数上限 `MAX_BOX_DEPTH` = 256**：嵌套更深的元素不再生成自己的盒（当 `display: contents`），子孙上移。防排版递归爆栈；测试 `deep_nesting_fits_in_one_megabyte_of_stack` 在 1 MB 栈的线程上跑 1500 层。

### 块排版（`layout/block.rs`，任务单第 10 节）

- 单位 `LayoutUnit` = 1/64 px 的 `i32`，加减乘都饱和在 ±2^24 px，1e9px 的恶意长度不会溢出。样式里的 `f32` 长度四舍五入换算；文字宽度按每个字形簇四舍五入到 1/64（测试字体里全是整数，不受影响）。
- 一个块只需要知道含块宽度（和已定的高度）就能排；它自己的位置由父亲事后定。外边距合并（CSS2 8.3.1）自下向上算：每个块报告上、下边缘露出来的外边距（已经和第一个/最后一个子块合并）和"自己的上下外边距穿过自己"（`through`）；父亲在兄弟之间合并并摆放。覆盖：兄弟、父子（上、下）、空块穿过、负外边距（最大正值加最小负值）、padding / border 隔断、新格式化上下文（`overflow` 非 visible/clip、`flow-root`、各种退路块、根元素）。
- 8.3.1 说盒子自己的上下外边距在 `height` 为 0 **或** `auto` 时才相邻（1b 审查后补 0）：`height: 0`（不论怎么写出来的、`min-height` 为 0、无边框内边距）的空块也穿过去；但"最后一个子块的下外边距并入父块"仍只对 `auto` 成立（`height: 0` 的父块里有内容时，子块的下外边距不并入）。
- 空块（`through`）的位置按 8.3.1 末段："像下边框不为零那样"——只算它自己的上外边距，所以 `g1 { margin: 20px 0 30px }` 夹在两块之间时，它在上一块下面 20px 处，而后一块在 30px 处。
- 宽度 10.3.3：auto 外边距、过约束（ltr：右外边距被忽略）、两个外边距都是 auto 时居中，装不下时左 0、右取余；min/max 10.4（算一遍，超过 max 用 max 再算，小于 min 用 min 再算）；`box-sizing: border-box` 的 width / min / max 先减去 padding + border（不小于 0）。`min-content / max-content / fit-content` 1b-1 当 `auto`，1b-2 起按内在宽度算。
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
- `text-align`：`left` / `start` / `match-parent` / `justify` / `justify-all` → 靠左（`justify` 的拉伸 1b-2 起做），`right` / `end` → 靠右，`center`。内容比行宽时不居中也不靠右，从行首开始（css-text-3 7.1）。没有右到左排版，`direction` 不影响。居中的偏移 = 空余 / 2（1/64 单位向零取整）。
- 文字片段（转储里的 `text`）的矩形是内容区：高 = 主字体上升 + 下降，底边不含半行距；同一行里同一个文字项的相邻单位合成一个片段。
- 1b-1 当时没做、1b-2 已做完的：`vertical-align`（只有 baseline）、行内盒的 padding / border / margin、`justify`、`text-indent`、`letter-spacing`、`word-spacing`、`text-transform`、装饰线、原子行内（`inline-block` 暂时按块）、图片、列表标记、定位、表单控件、**排版工作量计量**（现在只有 `MAX_BOX_DEPTH`；一个 10MB 没有空格的文字现在会整个整形，每个字符约占 100 字节）。

### `qinglan-cli layout` 和转储格式

- `qinglan-cli layout <文件> [--width 1280] [--height 1000] [--test-font] [--time]`。1b-3 起默认用 Windows 平台，`--test-font` 用测试平台。
- 转储格式（`layout/dump.rs` 开头有例子）：第一行 `viewport WxH`；之后每个块一行 `block <标签#id.类 | (anonymous)> (x,y WxH)`，坐标是页面坐标下的边框盒，单位 CSS px，去掉多余的 0，最小单位 1/64 = 0.015625；退路块后面加 `[display:flex]` 之类，`overflow: hidden/clip` 加 `[clip]`。块里有行内内容时，下面缩进列出 `line (x,y WxH) baseline=Y`（Y 是基线的页面 y）和它的 `text "…" (x,y WxH)` 片段，强制换行是 `br`（`<br>` 元素）或 `newline`（保留的换行符）。
- 快照：`tests/pages/*.html` 配 `*.layout.txt`。每个网页开头的 HTML 注释里写着按规范**手算**的关键数字，其中 `check: <行首文字> :: <期望片段>` 的行由测试程序逐条对照转储（手算数字不是从引擎抄来的，所以快照不会只是记录引擎当时的输出）；`viewport: WxH` 设视口。重新生成期望文件：`UPDATE_SNAPSHOTS=1 cargo test -p qinglan-engine --test layout_snapshots`，生成前先确认手算数字没有失败。

## 第 1 层 · 1b-2 行内细节、替换元素、列表、定位、表单、工作量计量

代码：`layout/` 下 `inline.rs`（重写）、`block.rs`、`boxtree.rs`、`intrinsic.rs`、`replaced.rs`、`position.rs`、`lists.rs`、`image.rs`、`dump.rs`；新属性 `tab-size`（`css/` 里，继承，数字或长度）。测试页在 `tests/pages/`，新增 12 个：`vertical-align`、`inline-edges`、`text-props`、`decorations`、`inline-block`、`images`、`srcset`、`lists`、`positioning`、`canvas-root`、`canvas-body`、`controls`；`fallbacks` 改写（inline-block 和 absolute 不再是退路）。每页开头的 `check:` 数字都是按规范手算的（算式写在页首注释里），引擎第一次跑出来和手算不同的只有三处，都是手算错了：`text-props` 的 "Hello World" 在 100 宽里要折行、`canvas-root` 的空 body 上下外边距穿过合并成 8、`lists` 的一个测试用字重名（改了测试，不是引擎）。快照测试的框架新增页首行 `dpr: 2`、`image: <id> WxH`（当作图片头已到）、`broken: <id>`。

### 盒子树的变化

- `inline-block` 和 `inline-flex/grid/table` → `BoxKind::InlineBlock`（原子行内盒，里面是块容器）；`img`、`iframe`、`video`、`audio`（有 `controls` 才显示，UA 样式表已有）、`canvas`、`object`、`embed`、复选框/单选框、`meter`、`progress`、`input type=range/color/image` → `BoxKind::Replaced`（叶子）。其余表单控件（文本框、`textarea`、`select`、`button`、`submit/reset/button/file` 的 `input`）是 `InlineBlock`（UA 样式 `display` 被作者改成块级时是 `Block`），文字由属性合成（`LayoutBox::synth`）：输入框的值（`password` 画成 •），没有值时灰色（#757575）的 `placeholder`；`select` 是被选中的（否则第一个）`option` 的文字；`submit`/`reset` 没有 `value` 时是 "Submit"/"Reset"；`file` 是 "Choose File  No file chosen"。单行文字不换行（`white-space: pre` + `nowrap`），盒子 `[clip]`。
- 不显示的 `<img>`（没有尺寸、没有加载）且有非空 `alt`：变成包着 alt 文字的行内盒（HTML 15.4.3：尺寸未知时当文字）；有宽或高（属性或 CSS）时仍是替换盒；加载失败或没有 `alt` 的是 0×0 替换盒。
- `position: absolute | fixed` 的盒子（`oof`）：留在父盒的子列表里，**一律当行内级占位**（`Out::Inline`）。这样它们不会像块级盒一样把行内元素劈开（CSS2 9.2.1.1 只管流内的块），只有占位的一段会生成匿名块（空匿名块高 0、外边距穿过，不影响别的盒子）。
- 列表项序号在建树时算（HTML 4.4.8）：同一个父元素一个计数器；`<ol start>`、`<ol reversed>`（没有 `start` 时从 `li` 子元素个数倒数）、`<li value>`（`ul`/`menu` 里也认）；`display: none` 的不计；不是 `li` 的 `display: list-item` 也计数（不认 `value`）。计数器只在建树时 O(n) 一遍。
- 文档序（`LayoutBox::order`，元素的先序编号，被块劈开的行内元素各段共用）用来排绘制顺序。
- `<details>` 一律展开：UA 样式表本来就不隐藏它的内容，只有 `summary` 的标记（`disclosure-closed`）改画成展开的三角 ▾（"disclosure-open" 同样）。

### 行内排版（`inline.rs`）

- 段落先 `collect` 成项（文字、强制换行、行内盒的起点/终点边、原子行内、定位占位），再 `prepare` 切成单元（文字一簇一个）；这一步不依赖宽度，所以量固有宽度时（`measure_inline`）和排行时（`lay_lines`）共用，量过的段落按块缓存（行内盒边距有百分比时因含块宽不同重做）。
- **行内盒的 padding / border / margin**：只有水平方向有值的行内盒才产生起点/终点"边"单元（宽度计入行宽，参与折行，终点边粘在前面的字上；断点处起点边归下一行）。被块劈开的行内元素只有第一段有起点边、最后一段有终点边（`SplitPart`）。每个有背景、边框或 padding 的行内盒每行出一个 `inline` 碎片：框 = 内容区（字体上升 + 下降）加上 padding 和 border，所以竖直的 padding/border 不撑行高；有起点/终点边的一行才带 `[first]`/`[last]`（给 1c 决定画不画那一侧的边框）。空的行内盒没有水平边就还是不参与行高。没有子盒的被劈开的段仍然丢掉（带边框的空段不保留，要保留得生成空碎片，第 2 层再说）。
- **`vertical-align`**：每行建一组"盒子条目"（行内盒、原子行内），算相对根（支柱）基线的偏移：`baseline` 0；`sub`/`super` 用 Blink 的式子（父元素字号 /5 + 1 往下，/3 + 1 往上；规范没规定）；`text-top`：盒顶对父元素内容区顶（偏移 = 父上升 − 盒的 A）；`text-bottom`：盒底对父元素内容区底（盒的 D − 父下降）；`middle`：盒中点对父基线上方 x 高的一半（x 高来自平台度量）；长度向上；百分比是元素自己行高（A + D）的百分比；`top`/`bottom`：先按其他盒子算出行高，比它矮就往下（top）或往上（bottom）撑到够高，再贴边。A/D 是上升/下降加半行距（`line-height` 的一半）。`top`/`bottom` 盒的后代只相对它定位，不参与行高。
- **原子行内**：`inline-block` 的基线是它最后一行的基线；`overflow` 不是 visible、没有行、替换元素、`textarea` 用下外边距边；文本框和 `select` 虽然内容被裁剪仍用文字基线。原子行内两侧都可断（不管周围是什么字；`nowrap` 里不断）。折行时先把整段里的原子行内排好，宽度才知道。
- **`text-align: justify`**（css-text-3 7.3, 8.4）：只对折行的行；最后一行和强制换行结尾的行不拉（`justify-all` 全拉）。机会：空格和 U+00A0 之后；中日韩字之间（后一个不是后置标点、前一个不是前置标点）。余量均分，除不尽的 1/64 单位给靠前的机会；字形数组里最后一个字形的前进宽度同步加上，所以只看字形数组就知道每个字画在哪。没有机会（比如一个长单词）不拉。挂在行尾的空格不算。
- **`text-indent`**：只第一行；百分比取块自己的内容宽；负值（悬挂）往左；制表位从内容盒左边算，不从缩进算；和 `center`/`right` 的关系：缩进后剩下的空间里对齐。`each-line`/`hanging` 关键字 1a 就丢掉了。
- **`letter-spacing`**：加在每个簇之后（包括行尾，和 Chrome 一样）；**`word-spacing`**：加在空格和 U+00A0 上；都直接加进字形的前进宽度。
- **`text-transform`**：折叠空白之后、整形之前改文字（所以转储里看到的是改后的文字）：`uppercase`/`lowercase` 用 Rust 的完整大小写映射（不处理土耳其语等语言特例）；`capitalize` 以字母数字为词，跨文字节点/元素连续（`h<b>ello</b>` 只大写 h）；`full-width` 把 ASCII 变成全角（空格变 U+3000）；`full-size-kana` 把小假名变大。
- **`tab-size`**（css-text-3 7.3）：数字 = 那么多个空格的前进宽度（含字距和词距）；长度直接用。
- **装饰线**（css-text-decor-3 2）：下划线在基线下 `underline_offset`（取整到整 px），删除线在基线上 `strikeout_offset`，上划线在内容区顶；粗细 = 字体给的粗细取整，至少 1px，`text-decoration-thickness` 优先；位置和粗细用文字碎片自己的字体。线跨整个文字碎片（含空格）。装饰从元素传到它流内后代的文字上（包括块级后代），到原子行内盒、定位盒就停（它们自己的声明照常对自己的内容有效）。同一种线同一颜色只画一次。每条线是 `Decoration`（`x, y, w, thickness`、样式、颜色），挂在文字碎片的 `decor` 区间上。
- **行内 `position: relative`**：偏移累加到它的碎片和里面的原子行内上（不影响别的盒）。

### 固有宽度、`min-content`/`max-content`/`fit-content`、收缩适应（`intrinsic.rs`）

- `content_widths(盒)` 给内容盒的 min/max-content（按盒缓存）：块容器 = 子盒外宽（margin box）的最大值；行内内容 = 同一份单元算出来的最长不可断段/不换行的最宽一行（含首行 `text-indent` 的固定部分）；文本框/`textarea` = 字符数 × "0" 的宽度；替换元素 = 它的used宽度。`outer_widths` 把固定的 `width`、`min/max-width`、padding、border、margin 算进去，**百分比一律当 `auto`/0**（循环百分比）。
- 关键字：`width/min-width/max-width` 的 `min-content`/`max-content`/`fit-content` 给的是**内容盒**宽度，和 `box-sizing` 无关；`fit-content` = min(max(min-content, 可用宽度), max-content)。
- 收缩适应（原子行内、绝对定位盒；CSS2 10.3.5、10.3.7、10.3.9）= 同一公式，可用宽度 = 含块宽 − 外边距 − padding − border；之后 min/max-width 再夹一次。

### 替换元素（`replaced.rs`）

- 尺寸：CSS2 10.3.2 / 10.6.2：`width`/`height` 都给了就用；只给一个的用宽高比算另一个（没有比例就用固有尺寸，再没有就用默认）；都是 auto 时用固有尺寸，并按 CSS2 10.4 的"约束违反表"（9 种情况）在 min/max 之间保持比例；宽高比 = 固有宽/高，没加载时用 `width`/`height` 两个属性（都有且 > 0）的比。默认尺寸 300×150（`audio` 高 54，`canvas` 取 `width`/`height` 属性，未加载的图片 0×0）；`box-sizing: border-box` 的宽高减去 padding 和 border。`iframe` 有 UA 的 2px 边框，所以框是 304×154。
- `check box / radio`：13×13（UA 样式表改成 `margin: 3px 3px 3px 4px`——1a 写成了 padding，不符合 HTML 15.5；这是 1b-2 对 `ua.css` 唯一的改动）；`meter` 80×16，`progress` 160×16，`range` 129×16，`color` 50×27。
- **图片 API（给 1c）**：`layout_document_with(doc, styles, platform, opts, &ImageSizes)`（`layout_document` = 没有任何图片信息）；`LayoutOptions { dpr, .. }` 决定 `srcset` 选哪张。`LayoutResult::images` 是 `Vec<ImageRequest { node, src, density }>`：每个有来源的 `<img>`/`<input type=image>` 一条，`src` 是页面里写的原文（1c 用文档的 base URL 解析），`density` 是这个来源的像素密度。1c 拿到图片头后 `ImageSizes::set_loaded(node, 像素宽, 像素高)`（失败 `set_broken(node)`，返回值是"有没有变化"）再重新排版；自然尺寸 = 像素 / density。按 DOM 节点编号做键，因为盒子编号每次排版都重建；1c 自己做节流和滚动锚定，可以用 `LayoutResult::page_pos(box)` 找视口第一个元素。没有 `ImageSizes` 条目的图片 = 还没到。
- **`srcset`/`sizes`**（HTML 4.8.4.3，`image.rs`）：候选 = `srcset` 各项 + `src`（没有 1x 候选时按 1x 加进去）；`w` 描述符除以 `sizes` 求出的来源宽（按顺序第一个条件成立的项，没有条件的项，默认 100vw；条件用 1a 的媒体查询求值，视口宽高 + dpr，长度用 `parse_length`，`calc()` 里不能有百分比）；`w` 和 `x` 同时写的项丢掉，描述符不认识的丢掉；最多 256 项。选法：密度 ≥ dpr 的最小者，没有就选最大者。**不做** `<picture>`/`<source>`，不看 `loading`（1c 可以自己读 DOM）。

### 列表（`lists.rs`、`block.rs`）

- 标记文字 = 计数器 + 后缀（css-counter-styles-3）：`disc` "• "、`circle` "◦ "、`square` "▪ "（符号也当文字，Windows 平台直接画那个字形；测试平台里每个字符 1em）；`decimal`/`lower-alpha`/`upper-alpha`/`lower-roman`/`upper-roman`/`lower-greek` "N. "；`decimal-leading-zero` "01. "；`cjk-decimal` "一〇、" 这类（后缀 、）；`none` 没有标记；字符串样式原样；不认识的计数器名按十进制；字母/罗马数字超出范围（≤ 0、罗马 > 3999）退回十进制；`disclosure-*` 见上。
- **外标记**：标记框右边贴着列表项的**边框盒**左边（CSS lists 3 的"外"）；竖直方向对齐列表项第一行的基线（沿子孙块往下找第一个行；找不到——空列表项——就放在内容区顶部，用标记自己字体的支柱，列表项高度不变）；用列表项自己的字体（所以大字号列表项的标记也大）。结果放在 `LayoutResult::markers`，`BoxOut::marker` 指过去，位置相对列表项边框盒（可以是负的）。**内标记**是该项第一个行内（文字碎片类型 `Marker`），列表项里全是块时（没有自己的行内内容）当外标记放。
- 不做 `::marker` 的样式（1a 只解析）、`list-style-image`（没解析）。

### 定位（`position.rs`、`block.rs`）

- **`relative`**：块和原子行内在被父盒摆放时加上偏移（`BlockResult::rel`），不影响流和外边距合并；`left` 优先于 `right`，`top` 优先于 `bottom`；百分比的竖直偏移只在含块高度明确时算。
- **`absolute`/`fixed`**（CSS2 9.6、10.3.7、10.3.8、10.6.4、10.6.5）：占位所在的行或匿名块给出**静态位置**（在行里 = 占位处的 x 和行顶；在匿名块里 = 块的内容区左上角），排队等包含块排完再排：含块 = 最近的定位祖先的 padding 盒（`cb_stack`），没有就是初始含块（视口大小，在页面原点），`fixed` 永远是视口。水平/竖直方程各 8 种自动值的组合照 10.3.7/10.6.4 写；auto 外边距两侧相等（水平方向装不下时左 0）；宽高 auto 且两侧都给了 → 填满；`width: auto` 收缩适应（可用宽度按规范用 `left`/`right` 剩下的）；`min/max-width` 用 10.4 的"算一遍、超了用边界再算"。排好后位置换算成相对静态位置所在块的边框盒（`LayoutResult::oof` 记着 `(块, 盒)`），所以转储和绘制都按普通的"相对父块"走。
- **行内 `position: relative` 的元素是它里面绝对定位盒的含块**（CSS2 10.1 第 4 条，1b 审查后补上；原先的"不当含块"取消）：行内排完一段后（`lay_lines` 末尾），含块 = 该元素**第一段**的左上 padding 角到**最后一段**的右下 padding 角（只做从左到右，和 Blink 旧排版一致：宽 = max(0, 最后一段的右 − 第一段的左)，高同理；规范 2.2 对多行的情形写"未定义"，包住第一段和最后一段的 padding 盒是 2.1 的写法，两者在换行的元素上不同，这里取前者，因为 CSS 2.0 原文和浏览器是这样）。padding 盒 = 边框盒去掉边框（边框只在元素的头尾两段上）；`relative` 的偏移算进去。元素在这一行没有碎片（里面只有占位，元素是空的）时，含块是占位处宽 0、高 = 元素字体内容区的盒。选择：元素里嵌了 `inline-block`、`inline-block` 里的绝对定位盒仍用最近的定位**块级**祖先（不往外传行内含块）；`fixed` 盒的坐标是滚动位置 0 时的视口坐标，1c 画的时候加滚动偏移；`sticky` 在 1a 已当 `relative`。
- **绘制顺序数据**：`LayoutResult::below`（`z-index` < 0，按 z 再按文档序）和 `above`（auto/0 按文档序，然后正数从小到大），元素是定位的块、原子行内、替换盒（`position != static`）。1c：先画 `below`，再画正常流，再画 `above`；`fixed` 的盒子 `position == Fixed`，用 `page_pos` 取位置再加滚动偏移。完整的层叠上下文留到第 2 层。
- **画布背景**（CSS2 14.2）：`LayoutResult::canvas = Some(Canvas { color, from })`：根元素有不透明背景就用它，否则用 `body` 的；`from` 那个盒子不再画自己的背景。`currentColor` 按那个元素自己的 `color`。

### 表单控件和占位框的尺寸

- 文本框：宽 = `size`（默认 20）× "0" 的宽度（元素自己的字体），再加 padding 和 border（UA 样式表是 `border-box`，但宽度是 auto 时不受影响）；高 = 一行 + padding + border。`textarea` = `cols`（20）×"0" 宽，`rows`（2）× 行高（内容多了也不撑高，裁剪）。`select`、`button`、`submit` 等收缩到文字宽。数值上限 1 万。不能操作，不画输入光标。
- 占位框里的文字（"iframe"、"video" 等）和灰色的底由 1c 画，布局只给出 `ReplacedKind`；`iframe` 的地址在 DOM 的 `src` 里（1c 点击时读）。

### 工作量计量和递归上限

- `Layouter::charge`：每个块计 4 + 子盒数，每段文字计字节数（整形前），每行计 4 + 行里的单元数，每个固有宽度计 2，每个定位盒计 8。预算 `DEFAULT_WORK_BUDGET` = 2000 万单位（`LayoutOptions::work_budget` 可改）。中文维基长条目用约 49 万（2.5%）；用完以后：不再排新的块和行，已经排好的保留；`LayoutResult::incomplete = true`（转储最后一行 `incomplete`），1c 在页面顶部显示"页面太复杂，只显示了一部分"。预算按"一个单位约 100 纳秒"选的：用满约 2 秒（release），留出余量给 1c 的 5 秒。
- **文字上限**：每页最多整形 `MAX_SHAPED_CHARS` = 100 万个字符（预算的一半更小时取一半）。一个字符在排版期间约占 130 字节（单元、字形各一份、断点标志等），所以 10 MB 没有空格的文字不会吃掉几个 GB：多出来的文字直接丢掉（已经整形的部分照常排），`incomplete = true`。中文维基长条目的文字量约为上限的五分之一。
- 嵌套深度：块嵌套仍由 `MAX_BOX_DEPTH`（256）管；原子行内/定位盒会经过 `layout_inline` → `lay_lines` → `layout_atomic` → `layout_block` 这条栈帧很大的链，所以另设 `MAX_PARAGRAPH_DEPTH` = 24：行内格式化上下文嵌套（或绝对定位盒嵌套）超过 24 层，更里面的内容不排（留空）。为了让 256 层纯块嵌套在 1 MB 栈（debug 主线程）里放得下，`layout_block` 拆成了几何计算（`block_geometry`）、内容、收尾（`finish_box`）三个函数，`dump_box` 也拆开了。三个测试在 1 MB 栈线程上跑 1500 层：块、`inline-block`/绝对/相对混合、纯块带定位。

### 性能

- 中文维基长条目，排版（测试字体、宽 1280、release，同一进程里连续 10 次）：1b-2 的中位数 **约 42.4 ms**（连续两次测量的中位数 42.3 和 42.4，每次 10 轮，最慢一轮 51.5），其中建盒子树 8.8 ms；同一台电脑、同一个测试程序跑 1b-1 的提交（461e3a7）是 32.2 ms（30.4–33.4）。慢了约 10 ms（+32%），目标（1b 验收）≤ 100 ms，达标。做过的优化（合计只省约 2 ms）：断点标志在整形后一次算好（不再每个单元二分查找）、原子行内的字段挪到旁表（单元更小）、装饰线继承链先用一遍按盒子的标志筛掉、字体缓存的键用指针（不再复制语言字符串）、没有水平边的行内盒直接跳过算边。剩下的差别分散在更大的盒子结构、每个文字碎片多做的垂直对齐记账上，没有再优化。（测量时没有确认是否插电；CLAUDE.md 要求插电测，1b-3 验收时要按插电重量。）新增测试 `perf` 里的 `zhwiki_layout` 会打印这个中位数、行数和工作量。
- `qinglan-cli layout` 新增的信息只在转储里，命令行没有新选项（`--dpr`/图片尺寸要到 1c 才有来源）。

### 没做 / 留给后面

- `<picture>`/`<source>`、`loading="lazy"`；行内元素的 `box-decoration-break`（一直按 `slice`）；`text-justify`、`text-align-last`、连字符；`vertical-align` 对表格单元格；`::marker`/`::before`/`::after`；`position: sticky` 的真实行为；行内元素做绝对定位含块；`float`（第 2 层）；`overflow: auto/scroll` 的滚动区域。
- 开放的问题：Windows 平台的 `x_height`、下划线位置等度量要在 1b-3 核对；`sub`/`super` 用了 Blink 的公式，Firefox 用字体表里的值，两者不同；`select` 的宽度按显示的那一项算，Chrome 按最宽的一项。

## 第 1 层 · 1b-3 Windows 平台（DirectWrite）和老编码

代码在 `crates/qinglan-win/`：`sys/mod.rs`（kernel32：`MultiByteToWideChar` 等）、`sys/dwrite.rs`（DirectWrite 的 COM 声明和安全封装）、`platform.rs`（`WinPlatform`，实现引擎的 `Platform`）、`encodings.rs`（编码名 → 代码页）、`lru.rs`（按字节计数的 LRU）、`fx.rs`（小哈希）。测试在 `crates/qinglan-win/tests/win.rs`（平台）和 `tests/corpus.rs`（真实页面）。

### COM 声明

- 全部手写，没有 `windows` / `windows-sys`。`unsafe` 只在 `sys/`：`sys/dwrite.rs` 和 `sys/mod.rs` 里的 `unsafe {}` 块（数量见报告），另有 1 个 `unsafe fn`（`ComPtr::from_raw`）、一组 COM 回调（`unsafe extern "system" fn`）和 3 个 `unsafe extern` 块；每个块都有 `// SAFETY:`。引擎 `forbid(unsafe_code)` 不变，`qinglan-win` 其余文件 `deny(unsafe_code)`；`qinglan-win` 也加了和引擎一样的 clippy 规则（禁 `unwrap`/`expect`/`panic`/下标越界）。
- 虚表只声明到用得到的方法，前面的槽位用 `_padN` 补。槽号按 Windows SDK 10.0.26100.0 的 `dwrite.h`、`dwrite_1.h`、`dwrite_2.h` 里方法的出现顺序逐个数过（`IUnknown` 占 0–2），写在 `sys/dwrite.rs` 开头：`IDWriteFactory` 3 GetSystemFontCollection、21 CreateTextAnalyzer、23 CreateGlyphRunAnalysis；`IDWriteFactory2` 26 GetSystemFontFallback（Factory1 占 24、25）；`IDWriteFontCollection` 4/5；`IDWriteFontFamily` 6/7（继承的 FontList 占 3–5）；`IDWriteFont` 3/10/13；`IDWriteFontFace` 8/10/11；`IDWriteTextAnalyzer` 3/6/7/8；`IDWriteGlyphRunAnalysis` 3/4；`IDWriteFontFallback` 3。接口号（IID）从同一份头文件抄。**核对办法**：用脚本从头文件里按顺序抽方法名再对照（脚本没提交）；运行时的核对就是测试——槽位错了会直接崩或返回垃圾，全部测试通过。
- 自己实现的 COM 对象只有 `IDWriteTextAnalysisSource` 和 `IDWriteTextAnalysisSink`：放在调用方的栈上，只在一次同步调用里使用，所以 `AddRef`/`Release` 什么都不做；`QueryInterface` 只认 `IUnknown` 和自己（DirectWrite 可能会问 `…Source1`，答不认识，它退回基础接口）。
- `DWriteCreateFactory` 直接要 `IDWriteFactory2`，所以系统要 Windows 8.1 以上；拿不到就 `WinPlatform::new()` 返回 `None`，CLI 报错退出（`--test-font` 仍可用）。

### 字体选择

- 通用字体族：`serif`/`ui-serif` → Times New Roman，`sans-serif`/`system-ui`/`ui-sans-serif`/`ui-rounded` → Segoe UI，`monospace`/`ui-monospace` → Consolas（任务单）；其余自定：`cursive` → Comic Sans MS（再试 Segoe Script），`fantasy` → Impact，`emoji` → Segoe UI Emoji，`math` → Cambria Math，`fangsong` → FangSong（再试 SimSun）。
- `font-family` 列表里系统有的字体按顺序组成"字体链"；一个都没有时用默认字体（Segoe UI，没有就 Arial、Tahoma、微软雅黑、宋体、集合里第一个）。
- 中文字体名：先用 `FindFamilyName`。本机实测 `微软雅黑`、`宋体` 直接找得到（和 `Microsoft YaHei` / `SimSun` 是同一个 `FontId`）。`FindFamilyName` 找不到时再试一张很小的对照表（微软雅黑 → Microsoft YaHei、宋体 → SimSun、黑体、楷体、仿宋、隶书、幼圆、微软正黑体、细明体、新细明体、标楷体，以及 メイリオ、ＭＳ ゴシック、맑은 고딕），因为 `FindFamilyName` 认不认本地化名字取决于那台电脑的字体带了哪些名字表。
- 粗细 1–999 取整后给 `GetFirstMatchingFont`（拉伸固定为正常），`Upright/Oblique/Italic` → 0/1/2。缺粗体或斜体时 DirectWrite 自己加模拟：测试里宋体粗体 `simulations & 1`、斜体 `& 2`，粗体遮罩的墨量比常规多；Segoe UI 有真粗体所以没有模拟。
- 语言：`lang` 规范成 `zh-Hans`（`zh`、`zh-CN`、`zh-SG`、`zh-Hans-*`）、`zh-Hant`（`zh-TW`、`zh-HK`、`zh-MO`、`zh-Hant-*`）、`ja`、`ko`；其他语言保留"语言-地区"；写得不合规矩的（非字母数字、过长）当没写。没有 `lang` 时用系统默认区域（`GetUserDefaultLocaleName`），再不行 `en-US`。**本机实测**：同样三个汉字"直骨海"，`zh-Hans` → Microsoft YaHei UI，`zh-Hant` → Microsoft JhengHei UI，`ja` → Yu Gothic UI，`ko` → Malgun Gothic；四种语言各选各的字体，`zh-CN` 和 `zh-Hans`、`zh-TW` 和 `zh-Hant` 结果相同。
- 回退按**字符**决定用哪个字体：先字体链里第一个有这个字符的（`GetGlyphIndices`，按字体缓存），再是系统之前为这个查询选过的回退字体（最多记 4 个，不用再问系统），最后才调 `IDWriteFontFallback::MapCharacters`（单个字符、带元素语言和主字体族名）。每个（查询，字符）的结果缓存。zhwiki 整页只调了 64 次 `MapCharacters`。`MapCharacters` 返回的缩放系数（`scale`）没用。控制字符、零宽/双向格式字符、组合记号、变体选择符跟着前一个字符的字体走。字体表上限 512 个（超了回退到主字体）。
- `FontId` = 字体表下标；同一个 `IDWriteFontFace` 对象（地址相同）只登记一次。测试 `shape_cache_hits_and_fonts_are_interned` 验证重新整形不会多出字体。

### 整形

- 一段文字按"字体"切成段；每段 `AnalyzeScript` 切成脚本段，每个脚本段 `GetGlyphs` + `GetGlyphPlacements`（自然宽度，不用 GDI 兼容宽度；不开从右到左，引擎没有 bidi）。可打印 ASCII（0x20–0x7E）的段直接用启动时对 "a" 分析一次得到的拉丁文脚本，省一次 `AnalyzeScript`。
- `clusterMap` 是 16 位，所以一次最多 8192 个 UTF-16 单元，更长的脚本段在不拆代理对的位置切开（切口处的字距调整丢掉，可以接受）。
- 簇的换算：`clusterMap` 值相同的连续字符是一个簇，拥有从它的值到下一个簇的值之间的字形；字形的 `cluster_start..cluster_end` 是这些字符的字节范围（UTF-16 位置 → 字节位置另外存了一张表）。测试检查簇不倒退、落在字符边界、首尾覆盖整个文字，含代理对和组合记号。
- DirectWrite 给制表符、换行符的是字体的缺字框（字形 0，宽度非零）；这两类和其他控制字符一律换成空格字形，宽度 0（制表符保留空格宽度；引擎自己按制表位算）。所以这些不算进缺字数。
- 长段在 8192 单元处切开时不切在簇中间（1b 审查后）：先在限度前 512 个单元内找空格，切在空格后；找不到就找最后一个"不拆代理对、后面不是组合记号/变体选择符/零宽连接符/韩文中后声、前面不是 ZWJ"的位置；512 内都没有才在限度处硬切。
- **偏离：汉字、假名、韩文音节、中日韩标点和全角标点（`is_context_free`，不含组合记号）按单个字符整形再缓存**（键：字体、语言、字符；在 100px 下整形一次，前进宽度按字号线性缩放）。这些字符默认没有字距调整、连字、上下文字形，所以结果和整段整形一致；省掉的是 DirectWrite 每次调用的固定开销（zhwiki 冷启动整形约 136 ms → 84 ms）。后面紧跟组合记号/变体选择符的字符不走这条路。取舍：汉字之间的 `kern`（字体里有也很少）不会生效。
- **冷启动整形加速（1b 审查后，`CacheStats::report` 里有拆分）**：
  - 计数：`qinglan-cli layout --time` 多打印整形调用数和时间、`AnalyzeScript`、`MapCharacters`、字体面创建、断行、词缓存命中。改前（zhwiki，冷启动，布局 ≈ 150 ms）：整形未命中的文字共 ≈ 95 ms，其中 `GetGlyphs`+`GetGlyphPlacements` 11 458 次 ≈ 79 ms（平均 7 µs），`AnalyzeScript` 3 480 次 0.3 ms，选字体 2.6 ms（`MapCharacters` 64 次 1.3 ms），建字体面 21 个 0.3 ms，断行 2 125 次 2.8 ms，其余（拼 UTF-16、簇换算、写缓存）≈ 10 ms。
  - **汉字等"上下文无关"字符成批整形**：`Platform::prefetch(chars, words, query)`（新增的、有默认空实现的提示方法）。排版开始前引擎扫一遍所有文字盒，按样式（字体、语言）收集不重复的非 ASCII 字符（按首次出现的顺序，因为回退字体的选择和之前选过什么有关）和不重复的 ASCII 词；平台把还没缓存的上下文无关字符按（字体、`AnalyzeScript` 给出的脚本）分组，每组 ≤ 2048 个字符拼成一个串，**每个字符后面跟一个空格**（两字符之间不会有字距调整），一次 `GetGlyphs`+`GetGlyphPlacements`；只有字形数 = 2×字符数且簇映射是 1:1 时才采用，否则什么也不缓存，退回原来的逐字符路径。结果和逐字符整形**逐位相同**：测试 `bulk_shaping_matches_single` 对 CJK 统一汉字全区、扩展 A、B 区抽样、兼容汉字、假名、韩文音节、CJK 和全角标点，在 5 种（字体，语言）下比较字形号、前进宽度（f32 位）、偏移、簇、字体族名。
  - **按词缓存**（Blink `CachingWordShaper` 的做法）：脚本段在每个 U+0020 处切开，词和空格各自整形（100px 下整形，使用时按字号线性缩放），键 = 字体、语言、脚本、文字；上限 6 万项，满了清空。**取舍：词和相邻空格之间的字距调整丢掉**（Times New Roman 里有"空格+A"这类字距对），和 Chrome 一样。
  - **ASCII 词成批整形**：`prefetch` 的词表里，字体能画全部可打印 ASCII 的，把一批词用 U+E000（私有区字符，字体里是缺字框，没有字距规则会越过它）隔开，一次整形，再按簇映射切出每个词。**只在结果和单独整形一样时才采用**：每个词的字形连续、后面正好跟一个分隔符字形、最后一个字形的前进宽度等于它的设计宽度（词和分隔符之间若有字距规则，只会改这一个值）；否则留给用到时再整形。测试 `bulk_word_shaping_matches_single` 用 4 000 个随机 ASCII 词加 "AV"/"To"/"WAVE" 等字距词，在 5 种字体下和单独整形逐位比较，并要求一半以上走了成批路径。
  - 改后（同机、插电、安静、A/B 交替各 7 次，release）：zhwiki 冷启动布局 **中位数 158 ms → 134 ms**。拆分：不命中的文字 ≈ 36 ms，成批整形 ≈ 42 ms（403 次调用，5 150 个词、2 874 个汉字），耗时超过 300 µs 的 26–28 次调用共 ≈ 18 ms（第一次用某个字体面+脚本时 DirectWrite 装表）。**没有达到 ≤ 100 ms**：剩下的主要是 DirectWrite 对约 3.5 万个字形的整形本身（成批和逐词的每字形耗时差不多，≈ 1.3 µs/字形），调用次数已经不是瓶颈。
  - 没做（要先确认）：在工作线程里并行整形（DirectWrite 的工厂、字体面是线程安全的，但 `WinPlatform` 现在是 `!Send`，要在 `sys/` 里给 COM 指针包 `Send`，属于新的 `unsafe`）；解析/样式期间预热字体面（同上）；只排视口附近的内容（引擎层面的改动）。
- 整形缓存：键 = 查询号 + 字号 + 文字（64 位哈希，命中后还会逐字比较文字，哈希碰撞不会出错）；值是字形数组；LRU，预算 **8 MB**，每项按"文字字节 + 每字形 28 字节 + 96"计。字形缓存：键 = 字体 + 字形号 + 字号 + 4 档，值是灰度遮罩（或"不画"），LRU，预算 **8 MB**，每项按遮罩字节 + 64 计。两个预算用测试 `caches_stay_inside_their_budgets` 检查；单项比预算还大就不存。

### 断行、度量、遮罩

- 断行：整个段落转 UTF-16，`AnalyzeLineBreakpoints` 一次，每个 UTF-16 单元一个字节（`DWRITE_LINE_BREAKPOINT`：低 2 位 before，2–3 位 after；0 中立、1 可断、2 不可断、3 必须断）。两个单元之间能断 = 两边都不是"不可断"，且至少一边是"可断/必须断"；代理对中间不断；返回字节位置。`word-break`/`overflow-wrap`/`white-space` 仍由引擎叠加。测试：空格后、汉字之间可断；逗号、句号前不断，逗号后可断。
- 度量来自 `DWRITE_FONT_METRICS`（设计单位 × 字号 / `designUnitsPerEm`）：上升、下降、行距（负的当 0）；x 高 = `xHeight`（0 表示字体没有，给 `None`）；"0" 的前进宽度 = "0" 字形的设计前进宽度（字体没有 "0" 给 `None`）。**符号约定**：DirectWrite 的 `underlinePosition`/`strikethroughPosition` 是线的上边相对基线、向上为正（下划线通常是负数）；接口要"下划线向下的距离、删除线向上的距离"，所以 `underline_offset = -underlinePosition`，`strikeout_offset = strikethroughPosition`。Segoe UI 100px：上升 107.9、下降 25.1、x 高 50、"0" 宽 53.9、下划线偏移 8.7（粗 5.8）、删除线偏移 25.9（粗 5.0）。1b-2 留下的"x_height、下划线位置要核对"到此核对过：测试里 "0" 的整形宽度 = `zero_advance`，其余量级合理。
- 遮罩：`CreateGlyphRunAnalysis`（渲染模式 NATURAL_SYMMETRIC，测量模式 NATURAL，笔位 x = 档数/4），取 ClearType 3x1 纹理（每像素 3 字节），三个条纹的平均值当灰度覆盖率。`left`/`top` 直接用纹理边界（已含 4 档的小数位）。宽或高超过 4096、字号超过 1024 不画。

### 老编码

- 引擎传来 WHATWG 编码名（不区分大小写）→ 代码页：`gbk`/`gb2312`/`gb18030` → 54936，`big5` → 950，`shift_jis` → 932，`euc-kr` → 949，`euc-jp` → **20932**（任务单没写；`MultiByteToWideChar` 对 51932 返回参数错误，本机实测，20932 能用），`iso-2022-jp` → 50220，`ibm866` 866，`iso-8859-2…8/13/15` → 28592…28598/28603/28605，`iso-8859-8-i` 38598，`koi8-r` 20866，`koi8-u` 21866，`macintosh` 10000，`x-mac-cyrillic` 10007，`windows-874/1250…1258`。**Windows 没有的**：`iso-8859-10`、`-14`、`-16`（返回 `None`，引擎退到 windows-1252 并给出警告）。
- 坏字节 → U+FFFD：先对整段用 `MB_ERR_INVALID_CHARS` 转一遍（合法的页面一次调用）；失败才分块重转（1b 审查后：原先整页逐字符重转，10 MB 的 GBK 加一个坏字节要 1.07 s，整转只要 0.16 s）：在小于 0x30 的字节前切成约 16 KB 的块（这些编码里小于 0x30 的字节从不是多字节字符的一部分：尾字节从 0x30 起，GB18030 四字节形式的第二、四字节是数字），干净的块一次转完；有坏字节的块对半再切、再试，小于 512 字节才逐字符转（双字节编码试 2 字节再试 1 字节，GB18030 在第二字节是数字时先试 4 字节），都不成才写 U+FFFD；连续失败超过 256 次就认定整页是坏的，剩下的直接逐字符转。结果和整页逐字符转完全一样（测试逐字比对）。无效的"首字节 + 非 ASCII 次字节"吃掉两个字节，其余只吃一个（ASCII 次字节还给后面，和 WHATWG 一致）。`iso-2022-jp` 的代码页不接受任何标志，直接转，坏字节由系统换成默认字符。
- 和 WHATWG 的已知差别：Windows 的单字节 0x80 在 Big5 里映射成 U+0080（标准是错误）；Big5 没有 HKSCS 扩展；GBK 用 GB18030 代码页（任务单规定），个别码位的映射和标准的 GBK 略有不同。这些在真实页面里极少出现，没有额外处理。

### 命令行

- `qinglan-cli style` 和 `layout` 默认用 Windows 平台（字体度量也进了 `ex`/`ch`；老编码页面、老编码样式表也靠它解码）；`--test-font` 回到测试平台。`dom` 用 Windows 平台只为了解码。`layout --time` 多打印一行 `.notdef glyphs after fallback: N`，N > 0 时附上前几个字符的码位。数的是页面上所有文字片段里字形号为 0（字体的缺字框）的字形。

### 验收数据

- 测试：见报告里的 `cargo test --workspace` 输出；平台测试 18 个（编码样例 GBK/gb2312/gb18030 四字节/Big5/Shift_JIS/半角片假名/EUC-KR/EUC-JP/windows-1251 和坏字节、字体族对应、中文字体名、拉丁/中日韩整形、回退字体、语言选字形、中英混排簇、控制字符、断行、度量、遮罩、粗体斜体模拟、缓存命中与预算、长文本分块），加 LRU 单元测试 3 个，加两个真实页面测试。
- 中文维基页面（`tests/corpus/local/zhwiki/`，宽 1280，release，**插电：`BatteryStatus` = 2**）：排版（不含解析和样式）**同一进程连续 10 次的中位数约 45–50 ms**；**第一次（冷，整形缓存是空的）约 150–190 ms**；`qinglan-cli layout` 单独跑一次（含字体加载）布局约 150 ms、样式约 37 ms。10 次里后 9 次靠整形缓存（缓存 4.4 MB，没超 8 MB 预算）。冷启动的 ~85 ms 花在 DirectWrite 的整形调用里（`GetGlyphs`+`GetGlyphPlacements`，约 10 000 次，每次 8 µs 左右），选字体只占 3 ms，`AnalyzeScript` 不到 1 ms。验收按任务单写的 10 次中位数算，达标（≤ 100 ms）；冷启动超过 100 ms，如实记下。（1b 审查后做了按词缓存和成批整形，见"整形"一节，冷启动布局降到约 134 ms，仍超过 100 ms。）
- 缺字框：中文维基页 139 932 个字形，**0 个** `.notdef`；GBK 老网页 4 690 个字形，**0 个**。
- GBK 老网页：`http://www.zuowen.com/`（作文网首页，2026-10-15 下载，142 KB，`<meta charset=gb2312>`，无 HTTP 头声明；用 GB18030 解码零个 U+FFFD），样式表 5 个，来自 `http://cmsimgs.zuowen.com/zuowen/…`（第 6 个样式表来自 `static-mmb.mmbang.info`，是第三方下载按钮用的，没下载）。放在 `tests/corpus/local/gbk/`（不提交）：`index.html` 原样；`local.html` 只把样式表链接改成相对路径、去掉第三方那条，用于命令行；`URL.txt` 记地址。引擎判断编码结果是 GBK，无警告。
- 本步没做：`MapCharacters` 返回的 `scale`；从右到左；`font-variation-settings`、`font-feature-settings`；颜色字体（表情只画黑白轮廓）；`@font-face` 网络字体（1c 之后）。

## 来源

- html5lib-tests：分词 `c777c408`（2026-09-29），建树 `9329e646`（2026-06-20，上游已搬到 WPT），MIT，见 `tests/html5lib/LICENSE`、`SOURCE.txt`。
- WPT `url/resources/urltestdata.json`，BSD-3-Clause，见 `tests/wpt-url/LICENSE.md`。
- 规范文本放在 `docs/spec/`（不提交），2026-10-15 下载。

### 1b 收尾：冷启动排版速度（2026-10-15）

- 1b 验收第 4 条（中文维基长条目排版 ≤ 100 毫秒）**按冷启动算没有达到**：每次新开进程量，整页排版中位数约 134 毫秒（优化前约 158 毫秒）；同一进程里热缓存的中位数约 45 毫秒，不代表用户第一次打开网页的情况。
- 剩下的主要是 DirectWrite 自己整形每个字形的开销（约 1.3 微秒 × 约 3.5 万个字形），加上第一次读字体表约 18 毫秒。
- 用户同意 1b 先到这里，首屏速度在 1c 解决：启动时在后台预热字体，第一次排版先排第一屏，1c 验收改量端到端的冷启动首屏时间（≤ 200 毫秒）。多线程整形留作备用办法。

## 第 1 层 · 1c-1 网络、录制回放、图片解码

代码：引擎（纯 Rust，`forbid(unsafe_code)`）里 `net/`（`mod.rs` 类型和 `Transport`、`policy.rs`、`loader.rs`、`cache.rs`、`content.rs`、`data_url.rs`、`file.rs`、`page.rs`、`blocking.rs`、`replay.rs`）、`lru.rs`（按字节计数的 LRU，响应缓存和解码缓存共用）、`platform/image.rs`（`ImageDecoder`、`DecodedCache`、`is_truncated`）、`css/import_scan.rs`、`css::linked_stylesheets`；`qinglan-win` 里 `sys/winhttp.rs`、`sys/wic.rs`（`unsafe` 只在这两个文件）、`net.rs`（`WinHttpTransport`）、`images.rs`（`WicDecoder`）；`qinglan-cli` 加 `record` 和 `--replay`，`dom`/`style`/`layout` 现在也能吃网址。测试：`crates/qinglan-engine/tests/net.rs`（假传输）、`crates/qinglan-win/tests/net.rs`（本地 HTTP 服务器 + WinHTTP）、`crates/qinglan-win/tests/images.rs`（WIC）、各模块里的单元测试；夹具在 `crates/qinglan-win/tests/fixtures/`（小图片和两个 gzip 文件，共约 110 KB，其中炸弹 100 KB）。

### 分层：策略在引擎，传输在平台

- **`Transport` 是阻塞的**：`fn fetch(&self, &Request) -> Result<Response, NetError>`，`Send + Sync`。任务单写"传输 trait 是异步的"，这里的异步放在 `Loader`：它自己开工作线程（每个请求一个线程，栈 1 MB，最多 16 个同时跑）调 `fetch`，结果通过 `mpsc` 通道送回，并调一次 `wake` 回调（1c-2 里是 `PostMessage`）。这样传输实现（WinHTTP、回放、录制）都很简单，并发上限、优先级、取消都只在一处。界面线程只调 `submit`（不阻塞）和 `poll`（不阻塞）；`wait(超时)` 给命令行和测试用。
- **`Loader`（`net/loader.rs`）**：`submit(LoadRequest { url, kind, page, use_cache }) -> Result<Ticket, Refusal>`（策略当场拒绝就是 `Err`，不会有事件）；之后每个 ticket 恰好一个 `LoadEvent { ticket, url, result: Result<Arc<Response>, NetError>, from_cache }`。HTTP 4xx/5xx 是 `Ok`（页面显示网站给的内容）。`cancel_all()`：清空队列、给运行中的请求置取消标志、之后到的旧结果一律丢掉。
- **并发和优先级**：每个主机（协议+主机+端口）≤ 6，总共 ≤ 16；等待队列分三档（页面 > 样式表 > 图片），同档先进先出，某主机满了就跳过它去启动别的；等待加就绪的请求超过 4096 个就拒绝（`Refusal::QueueFull`，防"10 万张图片"）。`file:` 和 `data:` 不占主机名额。
- **大小上限**由 `ResourceKind` 决定（页面、样式表 16 MB，图片 32 MB），放进 `Request::max_bytes` 交给传输在流式读取时执行；`Loader` 在传输返回后再核对一次（传输忘了也不会放过）。
- **取消的代价**：运行中的请求不能硬中断（WinHTTP 同步调用），只是每次读之间检查取消标志；结果被丢掉。

### 子资源规则（`policy::plan`，任务单第 13 节）

- `http`/`https`：都可以；**https 页面的 http 子资源**（样式表、图片）拒绝（`MixedContent`）。https 页面里点 http 链接（导航）不算混合内容。
- `data:`：只能做子资源（`Plan::Data`，不经过传输，`Loader` 当场解码后作为一个就绪事件交出）；当整页打开拒绝（`DataDocument`）。
- `file:`：用户从地址栏/命令行打开才行；**任何页面里的 `file:` 链接一律不跟**。**偏离/理解**：任务单写"网页里的 `file:` 链接和子资源一律不加载"。如果照字面做，本地保存的网页（包括 `tests/pages`、命令行的 `layout 某.html`）连自己的样式表和图片都读不到，所以做成：**本身是 `file:` 的页面可以加载 `file:` 子资源**（样式表、图片），http(s)/data 页面不行（`LocalFile`）。要改回字面意思只需改 `policy::plan` 里一行。
- `mailto:` → `Refusal::Mailto`（窗口弹确认再交给系统，1c-2）；`javascript:` → `Refusal::Javascript`（状态栏提示）；其他协议 → `UnsupportedScheme`。
- `data:` 自己解码（`data_url.rs`）：Fetch 的 "data: URL processor"，百分号解码用 1a 的 `percent_decode`，base64 是 Infra 的 forgiving-base64（去空白、末尾最多两个 `=`、长度模 4 余 1 失败、非法字符失败）；先按长度估算再分配，超上限报 `TooLarge`。MIME 为空时按规范补 `text/plain;charset=US-ASCII`。

### Referer 和请求头

- **Referer**（`policy::referer`，Referrer Policy `strict-origin-when-cross-origin`）：同源 → 页面完整地址（去掉用户名、密码、片段）；跨源 → 只有源（`https://example.com/`）；https 页面请求 http（降级）→ 不发；`file:`/`data:` 页面不发；完整地址超过 4096 字符改发源。"同源"按 `Url::origin()` 比（协议+主机+端口都得一样）。页面本身（用户输入的地址）不带 Referer。
- 其他请求头：`Accept`（按类型：页面、样式表 `text/css,*/*;q=0.1`、图片列出常见格式）、`User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) qinglan/0.1`（任务单；常量 `net::USER_AGENT`，由传输设置）、`Accept-Encoding: gzip, deflate`（WinHTTP 开了解压就自动加）。**1c-1 时不发 `Accept-Language`**（任务单没写；发了就会泄露系统语言）。**1c-2a 改为发**，见 1c-2a 一节。不发 Cookie、也不处理 `Set-Cookie`（WinHTTP 的 `DISABLE_FEATURE` 关了 Cookie）。

### 响应缓存（给前进后退）

- `net/cache.rs`：LRU，预算 **16 MB**，按响应体字节加头和地址的估算计；只存 2xx，`Cache-Control` 含 `no-store` 的不存；键是去掉片段的请求地址。**只有 `use_cache: true` 的请求才会读缓存**（前进后退用），普通打开和刷新走网络，但成功的响应总是存进去。单个响应比预算还大就不存。

### 非 HTML 响应（`net/content.rs`，任务单第 13 节）

- 按 `Content-Type` 的媒体类型分：`text/html`、`application/xhtml+xml` → HTML；`text/plain` → 纯文本（`blocking::open` 把它包成 `<pre>`，开头加一个换行，解析器会吞掉它，所以原文的首行换行不丢）；`image/*` → 图片单独显示（造一个只有 `<img src=原地址>` 的页面，图片响应已在缓存里，所以不会下两次）；其他 → `Unsupported { mime }`，窗口显示"第 1 层暂不支持下载"和网址。
- **没有 `Content-Type`** 时：看开头几个字节（PNG/JPEG/GIF/BMP/ICO/WebP 签名、SVG）→ 图片，否则当 HTML。`file:` 响应没有头，由 `net/file.rs` 按扩展名补 `Content-Type`（`html/htm/xhtml` → HTML，`txt/text/log/md` → 纯文本，`css`，图片格式），不认识的扩展名没有类型，走上面的嗅探，结果多半是 HTML。
- `application/octet-stream` 不嗅探，直接算"不支持"。

### 发现样式表和图片（`net/page.rs`）

- **样式表**：`css::linked_stylesheets(doc, url, opts)` 复用 1a 的 `collect_sources`（`<link rel=stylesheet>`，`media` 匹配，按文档顺序，相对 `<base>` 解析）。`@import` 用 `css::scan_imports(text)`：**不解析整张样式表**，只手工扫开头（空白、注释、`@charset`、`@layer a, b;`、`@import`），遇到第一条别的规则就停；认 `"x"`、`'x'`、`url(x)`、`url("x")`。这样 200 KB 的样式表也只花几微秒。引擎自己也只认开头的 `@import`，两边一致。
- 流程：`PageResources::start_sheets` 提交 `<link>`；每个到达的样式表用 `decode_stylesheet`（响应头 charset、BOM、`@charset`、页面编码）解码，再提交它的 `@import`（深度 ≤ 8，总数 ≤ `MAX_SHEETS` = 64，同一地址只取一次，所以 `@import` 循环自然停止）；`sheets_pending() == 0`（或窗口等了 5 秒放弃）后调 `style_document_with(.., &mut resources, ..)`——`PageResources` 实现 `ResourceLoader`，把取到的文本交给样式引擎，没取到的算 `missing`。失败原因（HTTP 404、混合内容、超时……）可用 `failed_sheets()` 查。
- **图片**：`LayoutResult::images`（1b-2 的 `ImageRequest`）→ `request_images`：`src` 按文档的 base URL 解析，同一地址在途时多个节点共用一次请求，同一节点同一地址不重复请求，**每页最多 2000 个不同图片地址**（超出的当坏图），策略拒绝的（混合内容等）立刻返回节点号让调用方标坏图。到达的图片以 `Update::Image { nodes, result }` 交出，调用方用 `probe_response`（SVG 直接 `Svg` 错误→`alt`；其余交给解码器的 `probe`）+ `apply_probe` 填 `ImageSizes`，再重新排版。
- `net/blocking.rs` 是同样逻辑的阻塞版（命令行、测试用）：`open`（取页面→分类→解码→解析）、`load_sheets`、`load_images`。窗口（1c-2）不用它，直接用事件。

### WinHTTP（`sys/winhttp.rs`、`net.rs`）

- 函数全部手写声明（`winhttp.h`，SDK 10.0.26100.0）：`WinHttpOpen/Connect/OpenRequest/SendRequest/ReceiveResponse/QueryHeaders/QueryDataAvailable/ReadData/SetOption/SetTimeouts/CloseHandle`。`Handle` 是带 `Drop` 的句柄包装，`unsafe impl Send/Sync`（WinHTTP 句柄可跨线程用，注释里写明）。
- 会话：`WinHttpOpen` 用 `AUTOMATIC_PROXY`（系统/WPAD/PAC），Windows 8.1 以前退回 `DEFAULT_PROXY`；`ENABLE_HTTP_PROTOCOL = HTTP2`（Windows 10 1607+，设置失败不致命）；`MAX_CONNS_PER_SERVER = 6`。请求：`DISABLE_FEATURE = COOKIES | REDIRECTS | AUTHENTICATION`（关 Cookie、自动重定向、自动认证；设置失败当错误，因为 Cookie 必须关）、`REDIRECT_POLICY = NEVER`、`DECOMPRESSION = ALL`（gzip+deflate 由 WinHTTP 解）、`WINHTTP_FLAG_ESCAPE_DISABLE`（地址已经是规范序列化，不要它再转义）、`CONNECT_RETRIES = 0`、`RECEIVE_RESPONSE_TIMEOUT`。
- **重定向自己跟**（不用 WinHTTP 的自动重定向）：每一跳都检查——最多 10 次（`TooManyRedirects`，重定向循环 11 次请求后停）、只去 http/https、**https → http 拒绝**（`BadRedirect`）、地址解析失败拒绝；`Response::url` 是最后一跳的地址（片段去掉）。
- **大小上限数解压后的字节**：`Content-Length` 超上限立刻拒绝（不读正文）；之后每读一块就累加，超了停下报 `TooLarge`。gzip 炸弹测试（100 MB 零，压缩后 100 KB）在 16 MB 处停止，不到 0.1 秒。
- **读正文要先 `QueryDataAvailable`**：只调 `WinHttpReadData(64 KB)` 时，一个每 100 毫秒发 1 字节的服务器会让这一次读一直等下去（实测，WinHTTP 同步读会等缓冲区填满），总超时就没用了。先问有多少数据（到数据就返回），再按这个数读，每次调用都很短，循环里每次检查总截止时间和取消标志，并把接收超时设成"剩余时间"。
- **超时**：连接（含域名解析）默认 15 秒，总共 60 秒；`WinHttpConfig { connect_timeout, total_timeout, max_redirects, user_agent }` 可配置（测试里用 1.5 秒）。**已知的怪事**：对一个收了请求就再也不回应的服务器，`WinHttpSendRequest` 本身要约 4.0 秒才返回（与设置的超时无关，实测总时限 1.5 秒和 3 秒都是 4.0 秒），之后 `ReceiveResponse` 立刻按超时失败；所以这种服务器在短总时限下会超出，默认 60 秒下无影响（最多比 60 秒多几秒）。原因没查清。
- **证书**：不设任何忽略证书的选项；`ERROR_WINHTTP_SECURE_FAILURE`（12175）和各种证书错误码（日期、名称、CA、吊销、用途、通道错误）映射成 `NetError::Tls(说明)`，没有"继续访问"。映射表在 `net::map_error`（有单元测试）；其他错误：12002 → `Timeout`，12007 → `Dns`，12029/12030/12031/12163 → `Connect`，12152 → `Protocol`，12156 → `BadRedirect`，12017 → `Cancelled`，其余 → `Other(代码)`。
- `unsafe`：本步新增在 `sys/winhttp.rs`（15 个块 + 2 个 `unsafe impl` + 2 个 `extern` 块）和 `sys/wic.rs`（21 个块 + 1 个 `unsafe fn` + 2 个 `extern` 块），每处都有 `SAFETY` 说明。

### 图片（WIC，`sys/wic.rs`、`images.rs`）

- COM 声明手写自 `wincodec.h`：`IWICImagingFactory` 第 4/10/11 槽（`CreateDecoderFromStream`、`CreateFormatConverter`、`CreateBitmapScaler`），`IWICBitmapDecoder` 第 13 槽（`GetFrame`），`IWICBitmapSource` 第 3/7 槽（`GetSize`、`CopyPixels`），转换器和缩放器的 `Initialize` 第 8 槽；槽号逐个数过头文件里的方法顺序，写在文件开头。字节流用 `SHCreateMemStream`（拷贝一份进 `IStream`），省掉 `IWICStream` 的声明。
- 线程：`WicDecoder` 无状态、`Send + Sync`，每次调用在本线程上 `CoInitializeEx(MULTITHREADED)`（线程局部守卫，线程结束时 `CoUninitialize`；线程已是 STA 时 `RPC_E_CHANGED_MODE` 不处理，WIC 两种都能用）并创建自己的工厂，所以能在工作线程上解码。测试 8 个线程各解 20 次。
- `probe`（只读头）：`CreateDecoderFromStream`（元数据按需）→ 第 0 帧 → `GetSize`；超过 **1 亿像素**（`MAX_IMAGE_PIXELS`，10000×10000 刚好允许）→ `TooLarge`，不解码。`decode(bytes, w, h)`：第 0 帧（GIF 取第一帧）→ `FormatConverter` 到 `32bppPBGRA`（预乘）→ 目标尺寸不同则 `BitmapScaler`（Fant）→ `CopyPixels`。转换和缩放是 WIC 管线上按需拉行的阶段，大图不会先整幅解出来。请求的目标尺寸超过 1600 万像素（64 MB）就按比例缩小。
- **只认完整文件**（`platform::image::is_truncated`）：WIC 对截断的文件很宽容（半张 PNG 照样"解"出来，测试里实测），任务单要求截断/损坏显示为坏图，所以解码前先做结构检查——PNG 要走完块到 `IEND`，JPEG 尾部 64 字节内要有 `FFD9`，GIF 要走完块到 `3B`，BMP/WebP 的文件长度不能短于头里写的大小；不认识的格式不判断。损坏的头由 WIC 报错：`UNKNOWNIMAGEFORMAT`/`COMPONENTNOTFOUND`/不支持的像素格式或操作 → `Unsupported`，其余 → `Corrupt`；两种都是坏图。
- **SVG**：`probe`/`decode` 直接 `ImageError::Svg`（不交给 WIC），页面显示 `alt`。**WebP**：交给 WIC，系统没装解码器时 WIC 报找不到组件 → `Unsupported` → 坏图；**没有 WebP 夹具**（没有能生成它的编码器），测试只覆盖"头对、数据是垃圾"的 WebP 被拒。ICO 取第 0 帧（不挑最大的）；GIF 的尺寸是第一帧的尺寸，不是逻辑屏幕的尺寸（多数 GIF 一样）。
- **解码缓存**（`platform::image::DecodedCache`）：键 = （地址，目标宽，目标高），LRU，预算 **24 MB**，按像素字节加 64 计，单个比预算大就返回但不存。放在引擎里（纯逻辑，用假解码器测）。1c-2 的懒加载、"离视口两屏才请求、四屏外丢掉解码结果"由窗口调 `remove` 实现，这一步没做。

### 录制和回放（`net/replay.rs`，任务单第 14 节）

- 目录里：每个响应体一个文件（`0001.html`、`0002.css`、`0003.jpg`……，扩展名按类型，未知 `.bin`）和 `index.txt`。索引是文本，块之间空行，`#` 注释；每块的键：`url:`（请求地址，回放按它查）、`final:`（重定向终点，没写就等于 `url`）、`status:`、`body:`（本目录里的文件名，没有这一行 = 空正文）、`header: 名: 值`（可以重复）。认不出的行跳过，同一地址多个块取第一个。手工改索引就能构造恶意响应（巨大的状态码、乱写的头、指向超大文件的 `body`）。
- `Recorder`（包一个真传输）：每个成功传输的响应存一份，**每个地址只存一次**；网络错误不存（回放时是 404）；**不存 `Set-Cookie`/`Set-Cookie2`**（个人数据，第 1 层也不用）和 `Content-Length`/`Content-Encoding`/`Transfer-Encoding`（正文存的是解压后的，这些头会说谎）。索引是追加写的，中途崩溃也留得下已有的。
- `ReplayTransport`：**结构上就不联网**——它里面没有别的传输；地址不在索引里 → 404（`text/plain`，"Not in the recording"）。`body:` 只认纯文件名（含 `/`、`\`、`:`、`..` 的拒绝，防止手改索引读到目录外的文件）；文件比 `max_bytes` 大 → `TooLarge`。
- 命令行：`qinglan-cli record <网址> <目录>` 走完整流程（页面→样式表和 `@import`→样式→排版→排版要的图片）再汇总；`--replay <目录>` 用在 `dom`/`style`/`layout`。`record` 用 1280×1000、dpr 1 排版来决定取哪些图片（`srcset` 的选择、`<img>` 列表）。**1c-2 的 `qinglan.exe --replay` 直接用同一个 `ReplayTransport`**。

### 命令行的变化

- `dom`/`style`/`layout` 的参数现在是"文件或网址"（`http://`、`https://` 开头的是网址）。文件也走新路径：`file:` 页面 + `PageResources`（样式表、`@import`、图片都经过策略和 `Loader`），所以本地页面里的图片现在有尺寸了（以前一律是"还没到"）。`layout` 先排一遍、取排版要的图片、有尺寸后再排第二遍。`--time` 多打印打开、样式表、图片各阶段的耗时。`style`/`layout` 的警告 `style sheet not found` 保留，另加 `style sheet not loaded: 地址 (原因)`。
- 页面类型：`text/plain` 的 `dom` 输出是 `<pre>` 的树；不支持的类型打印"不支持"并以失败退出。

### 测试

- 引擎（假传输，`tests/net.rs`）：协议/混合内容矩阵、Referer 矩阵、策略在传输之前（被拒的请求传输一次都没被调到）、`data:` 无传输、类型上限（含"传输忘了上限"）、每主机 6/总共 16（峰值计数）、页面和样式表插队、缓存（前进后退命中、刷新不命中、`no-store`/404 不存、预算、LRU 淘汰）、取消丢弃结果且 `wake` 被调、非 HTML 分类（html/纯文本/PDF/图片/`data:` 当整页被拒）、样式表和 `@import` 发现（循环、媒体不匹配、`alternate`、混合内容、各只取一次）、图片（同地址一次请求、SVG/404/坏图/混合内容/传输错误/坏地址都变坏图、`data:` 图片、重复调用不重取）、解码缓存、录制→回放（回放不调被录制的传输）、手改索引的路径穿越和超大文件。
- WinHTTP + 本地服务器（`qinglan-win/tests/net.rs`）：GET、响应头 charset、请求头（UA、Referer、Accept、Accept-Encoding）、Cookie 不发、gzip、**gzip 炸弹**、重定向链和**重定向循环**、重定向到 ftp 被拒、**永不结束的响应**（4 MB 处停）、**滴答式响应**和**不回应的服务器**（超时）、**Content-Length 1 亿**（立刻拒绝）、连接被拒、取消、`Loader`+WinHTTP 10 个并发请求带 Referer、真传输录制后关掉服务器再回放。
- WIC（`qinglan-win/tests/images.rs`）：PNG/JPEG/GIF/BMP 头尺寸、各格式解码颜色（PNG/BMP 精确，GIF ±8，JPEG ±60）、预乘（50% 红 → 0,0,128,128）、解到显示尺寸（400×200 → 40×20）和放大、**10 万×10 万的 PNG 头被拒**（`TooLarge`，刚好 1 亿允许、多一行拒绝）、截断的 PNG/GIF/BMP/JPEG、垃圾数据、SVG、假 WebP、把 PNG 每个字节翻转一遍都不 panic、8 线程并发、解码缓存计数。

### 没做 / 留给 1c-2、1c-3

- 窗口、绘制、懒加载（`loading="lazy"`、离视口两屏才请求、四屏外丢弃解码结果）、图片到达的重排节流和滚动锚定、`mailto:` 弹窗、错误页、状态栏提示、`qinglan.exe --replay`/`--bench`、`render`/`bench` 命令、真实网页库（30 个）和对比图。
- 没做 `<picture>`/`<source>`、`Content-Disposition`、`Content-Type` 以外的 MIME 嗅探、`Range`、HTTP 缓存语义（`ETag`、`max-age`）、代理认证、客户端证书（要客户端证书的网站是 `Tls` 错误）。
- HTTP/2 只是打开了选项，本地测试服务器说 HTTP/1.1，没有 HTTP/2 的测试；真实网站（维基百科）走的是 WinHTTP 协商的协议，没有单独确认是不是 h2。

### 验收数据（1c-1，2026-10-15）

- `cargo test --workspace`：170 个测试全过（引擎单元 57 + 集成 63 [css 15、html5lib 2、排版 17、net 13、perf 2、robustness 13、wpt_url 1]，win 单元 8 + 集成 42 [corpus 2、images 6、net 11、win 23]）；`cargo clippy --workspace --all-targets` 无警告。
- **恶意网络测试**（本地服务器，走 WinHTTP，debug 构建）：gzip 炸弹（解压后 100 MB，压缩后 102 KB）在 16 MB 处停，`TooLarge`，< 0.1 秒；重定向循环 11 次请求后 `TooManyRedirects`；永不结束的响应在 4 MB（测试里的上限）处停，`TooLarge`，< 0.1 秒；每 100 毫秒 1 字节的响应在总时限（测试里 1.5 秒）到时 `Timeout`；不回应的服务器 `Timeout`（实际 4.0 秒，见上面"已知的怪事"）；`Content-Length: 100000000` 立刻 `TooLarge`，不读正文；重定向到 `ftp:` 被拒。
- **图片恶意测试**：PNG 头声称 10 万×10 万 → `TooLarge`，不解码；截断的 PNG/GIF/BMP/JPEG、垃圾字节、假 WebP → 坏图；PNG 每个字节翻转一次（共 136 种）都不 panic。
- **真实页面**：`qinglan-cli record https://zh.wikipedia.org/wiki/中华人民共和国 tests/corpus/local/zhwiki-rec`（URL 是 `tests/corpus/local/zhwiki/page.html` 里的 canonical 地址；目录 git 忽略）：**146 个响应，正文共 4 840 683 字节**（目录 147 个文件，含索引共约 5.2 MB），实际联网约 20 秒（debug 构建，含排版）。HTTP 200，样式表 26 张（含内联）0 失败，图片 183 个节点：179 个拿到尺寸，4 个是 SVG（坏图，显示 `alt`）。`qinglan-cli layout <同一网址> --replay tests/corpus/local/zhwiki-rec`：退出码 0，转储 31 417 行，缺字框 0，整个进程 1.6 秒（debug；其中打开 188 ms、样式 258 ms、排版 514 ms、图片 166 ms、第二遍排版 324 ms）；两次录制各回放一次，转储逐字节相同。回放时断网也一样（回放传输里没有别的传输）。
- **证书**（联网抽查，不进测试）：`expired.badssl.com`、`self-signed.badssl.com`、`wrong.host.badssl.com` 都是 `NetError::Tls`（WinHTTP 12175），没有继续的办法。具体原因（过期/名字不对）WinHTTP 同步模式拿不到，只有 12175 这一个代码，所以错误页只能写"证书或 TLS 错误"；要给出具体原因得改用异步的 `WINHTTP_CALLBACK_STATUS_SECURE_FAILURE`，留作以后。

## 第 1 层 · 1c-2a 绘制、`render`、先排第一屏、预热、`Accept-Language`

代码：引擎 `paint/`（`mod.rs` 显示列表和空间索引、`build.rs` 生成、`raster.rs` 光栅、`images.rs` 懒加载和解码图片）；`layout/` 里加了"先排第一屏"（`LayoutOptions::stop_y`、`LayoutResult::partial`）和文字碎片的 `asc`；`platform::Platform::glyph_mask` 改成返回 `Rc<GlyphMask>`；`qinglan-win` 里 `sys/wic.rs`（PNG 编码，`write_png`）、`sys/dwrite.rs`（`warm`）、`prewarm()`、`Accept-Language`（`net::accept_language`、`ui_languages()`）；`qinglan-cli` 里 `render.rs`。测试：`crates/qinglan-engine/tests/paint.rs`（24 个）、`firstscreen.rs`（4 个）、`crates/qinglan-cli/tests/render.rs`（2 个）、win 里 `png_writer_round_trips_pixels`、`prewarm_opens_fonts_on_another_thread`、`accept_language_*`。

### 显示列表（`paint/build.rs`，任务单第 16 节）

- 列表和滚动位置、dpr、平台都无关：坐标是页面坐标（1/64 CSS 像素），每次排版建一份。内容：`Rect`（底色、边框的每一条边、装饰线）、`Image { node }`、`Text`（字形号、前进宽度、偏移、颜色、字号、起笔位置和基线）。每项带 `clip`（`DisplayList::clips` 的下标）。`fixed` 元素和里面的一切放进单独的 `fixed` 列表（视口坐标，滚动时不动）；`z-index < 0` 的 fixed 元素排在前面（`fixed_below`），画在正常流之前，其余画在最后。
- **绘制顺序**：先 `LayoutResult::below`（z < 0 的定位盒），再从根开始按树序画正常流（背景、边框、外标记、行：行内盒的底色和边框、文字、装饰线、原子行内；然后子块），最后 `above`。定位盒（`position` 不是 static/sticky）和它们里面的东西不在正常流里画，而是在自己那一轮画，里面再嵌的定位盒也各按全局顺序画（完整层叠上下文是第 2 层）。没有按 CSS2 附录 E 那样"所有块背景先、行内内容后"，而是树序（只在负外边距重叠时有差别）。
- **边框**：所有样式都画实线；每条边一个矩形，上下通长、左右夹在中间（不做斜角，相邻边颜色不同时转角归上下两条）；`none`/`hidden` 的宽度在计算值里已经是 0。行内盒的边框只在 `first`/`last` 那一行画对应的左/右边。装饰线都画成一条实线（`double`、`dotted`、`wavy` 不画特殊样子），画在文字之后。
- **裁剪**（`overflow: hidden/clip`，`BoxOut::clip`）：padding 盒，多层取交集，存成矩形。定位盒从祖先继承的裁剪：`absolute` 只受包含块（最近的定位祖先）及其以上的裁剪，中间的静态祖先不裁它；`fixed` 不受裁剪；`relative` 受所有祖先。（CSS2 11.1.1。）
- **替换盒**：图片画在内容盒（边框盒去掉边框和 padding；padding 百分比按父块边框盒宽近似）里，来源 `LayoutBox::image` 有值才出项目，位图有没有由光栅时问 `ImageSource`（没有就不画，坏图画空白，不画破图图标）；占位框（iframe 等）灰底 `#e0e0e0` 加左上角 4px 处的名字（用盒子自己的字体、灰字，裁在内容盒里）；复选框/单选框 13×13，`#767676` 1px 框、白底，有 `checked` 属性画深色内块（单选框也是方的）；`meter`/`progress`/`range`/`color` 画灰框灰底。
- `visibility: hidden` 的盒子自己的背景、边框、文字不画，可见的子孙照画；画布背景的来源盒（`Canvas::from`）不再画自己的底色。
- 页面高度：根盒底和所有（受裁剪限制的）项目底的最大值，封顶 2^22 px（防 `top: 1e9px`）。图片的位置（`ImageSpot`，给懒加载）和绘制位置（`ImagePlace`，给解码）也记在列表里。

### 空间索引和光栅（`paint/mod.rs`、`paint/raster.rs`）

- **索引**：页面按 256 px 切带，每个带里是和它相交的项目下标（升序）；横跨超过 8 个带的项目（整页背景）进 `tall` 表，每次查询都带上；查询把各带的下标合并、排序、去重，所以绘制顺序不变。滚动到 y=10000 的帧只碰 ≤ 12 个项目（`only_items_in_the_viewport_are_touched`：800 多个项目的页面）。
- **光栅**：先铺画布色（半透明的叠在白上），再画 `fixed_below`、索引查出的项目、其余 fixed。设备坐标 = CSS 坐标 × dpr，**每条边单独四舍五入**（相接的盒子在设备像素上也相接，边框保持整数宽）；滚动位置取整到设备像素。文字：基线取整，水平位置量化到 1/4 像素（和平台字形缓存的档位一致），字号 = CSS 字号 × dpr；遮罩直接和文字色混合（sRGB 空间里线性混合，没做对比度/伽马调整，字会比 DirectWrite 自己画的略细，以后看效果再说）。图片：位图尺寸正好等于目标矩形时逐行拷贝（正常情况：按显示尺寸解码），否则双线性（8 位权重，边缘夹住）；预乘 BGRA 的 source-over。
- 光栅不解码、不联网：位图由 `ImageSource::bitmap(node, 宽, 高)` 给，没有就跳过。`glyph_mask` 现在返回 `Rc<GlyphMask>`（和平台的 LRU 共用，画同一个字形两次不再复制遮罩）。

### 图片懒加载和释放（`paint/images.rs`，任务单第 15 节）

- 都是视口高度的倍数（"一屏"）：**请求**——普通图片离视口 ≤ 2 屏就请求，`loading="lazy"` 的 ≤ 1 屏（任务单没写"近"是多近，这是我定的：普通图片的"近"就是它本来不该被推迟的距离，懒加载的更紧）；`render --full`/`load_all` 全部请求。**解码**——离视口 ≤ 1 屏的图片按显示尺寸（设备像素）解码，光栅前做，光栅本身不解码。**释放**——离视口 > 4 屏的解码结果丢掉（`ImageStore::drop_far`），压缩字节留在加载器的响应缓存里，回到近处时再从缓存解码（`prepare`）；缓存里已经没有字节（被 16 MB LRU 挤掉、或 `no-store` 的响应根本没存）的，`prepare` 在 `missing` 里报出来，窗口重新请求。
- `wanted_images(layout, list, band, load_all)`、`far_images(list, band)` 是纯函数，窗口每次排版和滚动后调；`PageResources::request_images` 本来就跳过已请求的节点，所以可以反复调。图片没有尺寸的排版里（0×0）它们都挤在页面上方，所以刚开始请求的比最终排版里"近"的多，已发出的请求不收回（Chrome 一样）。先排第一屏的结果里，停止线以下还没排的图片位置未知，一律算"远"，整页排完再算。
- 解码缓存仍是 24 MB LRU；`ImageStore` 另记每个节点的解码键，按节点丢。解码失败的节点不再重试（`failed`）。

### 先排第一屏（`layout/`，任务单第 17 节引擎部分）

- 选项 `LayoutOptions::stop_y: Option<f32>`（CSS 像素，页面坐标；窗口传 滚动位置 + 视口高 + 一屏，也就是 `scroll + 2 × 视口高`；要跳到 `#片段`/恢复滚动位置就把 `scroll` 换成目标位置）。块的子盒循环里，排每个子块之前估计它的页面 y（父块的估计 y + 光标 + 待合并的外边距）；估计 ≥ 停止线就停，这个块和后面所有子块都不排，`LayoutResult::partial = true`。行内排版里同样按行（估计 y = 块的估计 y + 内容区顶 + 行顶）。估计值是真实位置的下界（子块的外边距还可能与父块合并而下移），所以停得略晚，不会早。
- **保证和测试**（`tests/firstscreen.rs`）：对每个起点在停止线以上、且不在脱离文档流盒里的盒子，x、y、宽和完整排版一样；完全落在停止线以上的盒子，高、裁剪、每一行和每个碎片（x/y/宽/高/所属盒）、有没有标记都一样。跑了 `tests/pages/` 的 28 个页面 × 3 条停止线（1/4、1/2、3/4 页高）和 zhwiki-rec（1920×1080，停止线 2160），全部一致。
- 取舍：被停止线切到的块（祖先链）高度不是最终值，它们里面的 `absolute`（包含块是这些块）的盒子**不排**，等整页；原子行内（`inline-block`、图片）和脱离文档流的盒子（`absolute`/`fixed`）总是排完整（`no_stop` 计数），因为排它们时位置还不知道。**已知的缺口**：`absolute`/`fixed` 盒子如果在文档里的位置在停止线之后（比如 `<body>` 末尾的悬浮条、回到顶部按钮），第一遍里没有它们——它们的占位还没排到，静态位置未知，包含块（初始包含块/视口）在上面也没用；整页排完再出现。要避免这个闪一下，得在第一遍扫一遍定位元素并把"`top`/`left` 都不是 auto"的先排掉，这需要知道占位所在的块的坐标，没做。
- 文字预取：第一遍里只预取页面前面 `停止线 × 视口宽 / 60` 字节的文字（再加 4 KB），不是整页（整页的批量整形 ≈ 28 ms，冷）。预取只是提示，不改变结果。
- 之后的整页排版就是不带 `stop_y` 再调一次 `layout_document_with`（整页重排）。

### 预热（`qinglan-win` 的 `prewarm()`）

- 建 DirectWrite 共享工厂、系统字体集合、回退、文字分析器；打开 Segoe UI、微软雅黑、宋体、Consolas、Times New Roman（常规、粗体各一，装了的才开），对每个字体整形一段样例（拉丁或中文）并画一个字形，再用 `MapCharacters` 走一次中文回退。建好的对象放进进程里的静态量（`OnceLock`，`Warm` 的 `Send`/`Sync` 在 `sys/dwrite.rs` 里用 `unsafe impl` 并写明 DirectWrite 对象是自由线程的），所以 DirectWrite 共享工厂里的缓存一直是热的；`WinPlatform` 之后新建（任何线程）直接用。返回打开的字体数（第二次调用返回 0）。不碰任何 `WinPlatform`，网页先到就先排，不等它。
- **测量**（`render --time`，回放 zhwiki，预热在内容到之前跑完）：预热本身 4.5–7 ms（偶尔 13 ms）；`--no-prewarm` 对比：首屏中位数约 79–80 ms → 76 ms（省约 3 ms，3–4%）。比 1b 收尾时记的"首次读字体表约 18 ms"小得多：那时系统的字体文件缓存是冷的；现在每次新开进程，操作系统的文件缓存已经热了（量不到重启后第一次的情形）。所以预热在这台机器上收益很小，留着是因为开销也小（后台线程 5 ms）；重启后第一次打开的收益没有量到。

### `Accept-Language`（任务单第 13 节，**取代** 1c-1 写的"不发"）

- `WinHttpTransport` 每个请求带 `Accept-Language`，由 `GetUserPreferredUILanguages` 的第一个界面语言生成（取不到用用户区域名）：`zh-CN` → `zh-CN,zh;q=0.9,en;q=0.8`，`en-US` → `en-US,en;q=0.9`，`ja` → `ja,en;q=0.9`；请求自己已经带了这个头就不加；`WinHttpConfig::accept_language` 可以覆盖（测试用）。1c-1 写的理由"发了就泄露系统语言"仍然成立，但任务单明确要它（中文维基靠它选简体/繁体），以任务单为准。重放不涉及。

### `qinglan-cli render`（任务单第 18 节）

- `render <网址或文件> -o out.png [--width 1280] [--height 1000 | --full] [--dpr 1] [--scroll Y] [--replay 目录] [--test-font] [--time] [--scroll-bench] [--no-prewarm] [--no-first-screen-first]`。流程是窗口要走的：样式表到齐 →（`--time` 时先量首屏）→ 样式 → 排版 → 取近处的图片（`--full` 取全部）→ 有尺寸后重排（最多 4 轮，到没有变化为止）→ 解码近处的位图 → 光栅 → 用 WIC 写 PNG（24 位，半透明的叠在白上，分 64 行写入）。
- `--full`：画面高 = 页面高，`fixed` 元素画在顶部（滚动位置 0）；总像素超过 3200 万（128 MB）就截断并提示。普通模式宽高×dpr 超过同样上限直接报错退出。`--scroll Y` 在普通模式有效。
- `--time`：先量"内容到齐 → 首屏像素在缓冲区里"（样式 + 停止线排版 + 显示列表 + 光栅，没有图片，和窗口第一次画的一样），再做完整流程并打印各段；`--scroll-bench`：从头到尾每帧滚 100 px，每帧只量 `render_frame`（图片解码在帧之前，不计入），第 1 遍是每个字形第一次出现（平台遮罩缓存边走边填），第 2 遍是热的。
- PNG 编码的 COM 声明手写在 `sys/wic.rs`：`IWICImagingFactory::CreateEncoder`（槽 8）、`IWICBitmapEncoder`（3 Initialize、10 CreateNewFrame、11 Commit）、`IWICBitmapFrameEncode`（3 Initialize、4 SetSize、6 SetPixelFormat、10 WritePixels、12 Commit），槽号对照 `wincodec.h`（SDK 10.0.26100.0）数过；输出用 `SHCreateStreamOnFileEx`。新增 `unsafe` 块全在 `sys/` 里（`wic.rs` 13 个、`dwrite.rs` 2 个 `unsafe impl`、`mod.rs` 2 个），都有 `SAFETY` 说明。

### 验收数据（1c-2a，2026-10-15，release，插电 `BatteryStatus = 2`）

- **环境**：插电（`BatteryStatus = 2`）；Intel i7-8850H；测量时另一个会话在编译/跑 qingpdf 的测试（`cargo`/`rustc` 进程在，总 CPU 10–15%），别的是 Chrome、Claude 桌面版、Malwarebytes。有一次 CPU 被另一个会话占满（100%），那几轮数字慢了一倍，已作废重量。回放 `tests/corpus/local/zhwiki-rec`（146 个响应），1920×1080，dpr 1。
- **冷启动首屏**（每次新开进程，预热在内容到齐前跑完，先排第一屏开着；"内容到齐"= 页面和 `<head>` 里的样式表都到了、预热和平台已建好；"首屏像素在缓冲区里"= 样式 + 停止线以内的排版 + 显示列表 + 光栅，没有图片，不含贴到窗口）：5 次 82.2、82.4、82.7、85.0、86.3 ms，**中位数 82.7 ms**（目标 ≤ 200 ms 减去贴窗口的时间，余量很大）。分段（中位数附近）：样式 38 + 排版 40 + 显示列表 0.6 + 光栅 4.7。再往前一轮同样 5 次：80.3、85.9、79.2、88.6、78.3，中位数 80.3。
  - 对比：关掉先排第一屏（`--no-first-screen-first`）：排版 118 ms、显示列表 5 ms，首屏 160–162 ms；所以先排第一屏省约 78 ms（排版 118 → 40）。关掉预热（`--no-prewarm`）：79–80 ms，对 76 ms，约 3 ms。限制预取之前（整页预取）首屏是 88 ms（排版 49 ms），限制后排版 38–40 ms。
  - 整页：完整排版冷启动约 118 ms（同 1b 收尾的 134 ms 同量级），窗口在首屏画完后马上接着排；`render` 的"取图片 + 整页排版（2 轮）+ 显示列表"约 270–350 ms（含从回放取 183 个图片里近处的头和一次重排）。
- **滚动光栅**（`--full --scroll-bench`，所有图片都取了，每帧滚 100 px，711 帧，只量 `render_frame`，不含图片解码）：dpr 1：中位数 **4.96 ms**、p95 **8.14 ms**、最大 14.8 ms（第 1 遍，字形第一次出现）；第 2 遍（热）4.49 / 5.70 ms。dpr 1.5：中位数 **10.16 ms**、p95 **14.27 ms**、最大 22.4 ms；热 9.50 / 11.60 ms。目标是中位数 ≤ 16 ms、p95 ≤ 33 ms，都达标；每帧约 370 个项目、2000 个字形。不带 `--full`（只取近处的图片）同样：dpr 1 为 4.4 / 6.7 ms，dpr 1.5 为 10.7 / 15.8 ms。
- **测试**：`cargo test --workspace`：203 个测试全过（引擎单元 57 + 集成 91 [css 15、firstscreen 4、html5lib 2、排版 17、net 13、paint 24、perf 2、robustness 13、wpt_url 1]，cli 集成 2，win 单元 9 + 集成 44 [corpus 2、images 7、net 11、win 24]；1c-1 是 170 个）；`cargo clippy --workspace --all-targets` 无警告。新增：paint 24 个（矩形/边框/三种边框样式/裁剪/嵌套裁剪/文字/半透明文字/装饰线/dpr 1、1.5、2/图片 1:1 和双线性/半透明图片/fixed 滚动后/z 序/负 z 的 fixed/absolute 的裁剪规则/画布背景/hidden/标记和复选框/索引只碰视口内的项目/恶意数值/先排第一屏与整页首屏逐像素相同/懒加载/图片位置未定/解码和释放和重新解码和缺字节）、firstscreen 4 个、cli 2 个、win 3 个（PNG 往返、预热、`accept_language` 单元测试；`Accept-Language` 头的断言加在已有的 net 测试里）。

### 没做 / 留给 1c-2b、1c-3（1c-2b 已做完，见下一节）

- 窗口、把帧贴到窗口上、滚动时"平移旧画面、只画新露出的部分"（目前每帧都是整个视口重画，已经够快所以没做）、图片到达的重排节流和滚动锚定、`qinglan.exe --bench`、`qinglan-cli bench`。
- 图片的 `object-fit`、破图图标、`alt` 文字（有尺寸的坏图画空白）、`<picture>`；单选框画成圆的；圆角、渐变、阴影（任务单"不做"）。
- 开放问题：文字对比度（线性混合偏细）；先排第一屏的"末尾悬浮条"缺口（见上）；预热收益只在热缓存下量过。

## 第 1 层 · 1c-2b 窗口、`qinglan.exe --bench`、`qinglan-cli bench`

代码：`qinglan-win` 里 `sys/window.rs`（user32/gdi32/kernel32/shell32 的声明和安全包装，`unsafe` 只在这里）、`window.rs`（把窗口消息变成 `Event`，给应用一个安全的 `Ui`）；引擎里 `interact.rs`（点链接、`#片段`、元素位置、滚动锚点、标题）、`net/blocking.rs` 的 `open_response`、`PageResources::{forget_images, abandon_pending}`；`crates/qinglan`（`#![forbid(unsafe_code)]`）里 `app.rs`（浏览器逻辑）、`page.rs`（一个页面的 DOM/样式/排版/显示列表）、`address.rs`、`history.rs`、`throttle.rs`、`i18n.rs`、`errorpage.rs`、`bench.rs`、`args.rs`；`qinglan-cli` 加 `bench`。两个 exe 的 `build.rs` 都传 `/STACK:8388608`（用 PE 头核对过：两个都是 8 MB，`qinglan.exe` 是 GUI 子系统，`qinglan-cli.exe` 是控制台）。测试：qinglan 单元 32 个（地址栏分类、历史和滚动恢复、重排节流、错误页、中英文字符串表、命令行、bench 报告）、`tests/bench_smoke.rs`（3 个，真开一个窗口跑 `--bench`）、引擎 `tests/interact.rs`（6 个）、cli `tests/bench.rs`（2 个）、win 的 `sys/window.rs` 里结构体大小和进程数字两个单元测试。

### 窗口结构（`qinglan-win/src/window.rs`）

- 一个顶层窗口（`QinglanFrame`，`WS_CLIPCHILDREN`）加子窗口：三个 `BUTTON`（← → ↻/✕，Segoe UI Symbol）、地址栏（系统 `EDIT`，所以输入法直接能用）、提示条（`STATIC`，淡黄底，平时隐藏）、页面区（`QinglanPage`，带原生竖直滚动条）、状态栏左右两个 `STATIC`（左：链接地址/加载进度；右：http 页面显示"不安全"）。字体取系统消息字体（`SPI_GETNONCLIENTMETRICS` 按 DPI），所以中文系统自然是微软雅黑 UI。
- **滚动条用原生的、始终显示**（`SIF_DISABLENOSCROLL`）：页面高度变化时可视宽度不变，不会因为出现滚动条而重排；拖拽用 `SIF_TRACKPOS` 取 32 位位置。
- **消息 → 事件**：窗口过程（`extern "system"`，在 `sys` 里）把消息交给 `window::dispatch`，那里转成 `Event`（`Size`、`Paint`、`MouseMove`、`Click`、`Wheel`、`Key`、`Scroll`、`Command`、`Timer`、`Wake`、`User`、`Ready`、`Close`、`Dpi`）交给 `App::event`。应用处理事件时窗口自己产生的消息（改大小引起的 `WM_SIZE`、定时器、唤醒）进队列，处理完再交；鼠标/按键/绘制这类只有"当下"才有意义的丢掉。`catch_unwind` 包在 `dispatch` 和每次 `App::event` 外面（panic 不能穿过窗口过程）。
- 地址栏用子类化（`SetWindowLongPtrW(GWLP_WNDPROC)`）接按键：Enter、Esc、Ctrl+L/F6/N/R、F5、Alt+←/→ 在地址栏里也生效；Enter/Esc 的 `WM_CHAR` 吞掉（否则嘟一声）；Ctrl+A 全选自己处理。**输入法**：`SendKeys` 在这台机器的中文输入法下，字母会走输入法组合串，Enter 先被输入法用来"上屏"，所以自动化测试里用 `WM_SETTEXT` 加直接发 `WM_KEYDOWN`；真人在英文状态下输网址、Enter 直接打开，中文组合串状态下第一个 Enter 是上屏（和所有浏览器一样）。
- **唤醒**：`Loader` 的 `wake` 回调 = `Waker::wake()`，`PostMessageW(WM_APP+1)`；多次唤醒合并成一条（`pending` 原子标志，界面线程处理前清掉）。界面线程只调 `Loader::poll`（不阻塞）。
- **DPI**：启动时 `SetProcessDpiAwarenessContext(-4)`（per-monitor v2），`GetDpiForWindow` 取 DPI，`dpr = dpi / 96`；`WM_DPICHANGED` 里按系统建议的矩形移动窗口、重做字体、重新摆控件，再发 `Event::Dpi`（应用重新样式+排版，解码尺寸变了的图片靠 `ImageStore::prepare` 自己换）。**没有在多显示器上实测 `WM_DPICHANGED`**（只有本机一个 125% 的显示器），列进手工检查清单。
- **`println!` 在 GUI 进程里**：`#![windows_subsystem = "windows"]`（`cfg(not(test))`，不然 `cargo test` 的输出也没了）；`--bench` 或命令行有错时 `AttachConsole(ATTACH_PARENT_PROCESS)`，stdout 没被重定向（句柄是 0 或 -1）就把 stdout/stderr 指到 `CONOUT$`。重定向到管道/文件时句柄本来就有效，不动它。
- 新窗口 = 新进程（`std::process::Command` 启动 `current_exe()`，带 `--replay` 原样传下去，不需要 `unsafe`）。`mailto:` 弹 `MessageBoxW` 确认后用 `ShellExecuteW`。

### 地址栏（`address.rs`，任务单第 17 节）

带 `scheme://` 的原样打开；`mailto:`/`data:`/`about:`/`javascript:`/`file:`/`view-source:`/`tel:`/`blob:` 不带斜杠也算协议；`C:\...`、`C:/...`、`\\server\share` 变 `file:` 地址（这是任务单没写的补充，粘贴路径很常见）；没有空格、主机部分（去掉用户名、端口、路径）是 `localhost`、含点（且不以点开头）或 IPv6 字面量的补 `https://`；其余是搜索词，`https://html.duckduckgo.com/html/?q=` 加百分号编码的查询（空格变 `+`，非 ASCII 按 UTF-8 编码）。`localhost:8080` 不会被当成协议 `localhost:`。"像主机名"规则是任务单的：`node.js`、`v1.2` 这样的搜索词会被当成网址，这是规则本身的代价。

### 页面加载流程（`app.rs`、`page.rs`）

1. **导航**：先问 `policy::plan`（`mailto:`、`javascript:` 等拒绝的当场处理，**不取消**当前页的加载）；再 `cancel_all`、提交页面请求（`use_cache` = 前进后退）。旧页面在新页面的字节到了、解码解析之前就整个丢掉（`self.page = None`，DOM、样式、排版、显示列表、解码图片一起释放），然后才建新页面。
2. **HTML 到齐后**：`blocking::open_response`（新拆出来的"拿到响应以后"那一半：分类、解码、解析）→ 提交 `<head>` 的样式表 → 样式表都到了（或 **5 秒定时器**到点，没到的当 `missing`，提示条说有几个没加载）就开始显示。没有流式解析：HTML 整个到了才解析（所以"内容到齐"的时钟包含解析，见 bench）。
3. **先排第一屏**：样式 → `stop_y = 滚动位置 + 2 × 视口高` 的排版 → 显示列表 → 光栅 → 贴窗口（第一屏）；然后马上请求第一屏附近的图片，同一次事件里接着整页排版（界面线程上，期间不处理消息，所以"整页没排完不让滚到没排的地方"自然成立）。**`#片段`的页面不做先排第一屏**，直接整页排（目标位置要排完才知道；任务单写"排到目标位置再多一屏"，这要排版选项能"排到某个节点"，没做——偏离）。恢复滚动位置的（前进后退、刷新）用保存的位置当起点，**保存的位置优先于 `#片段`**。
4. **图片**：近处的（`wanted_images`，按 1c-2a 的规则）经 `PageResources::request_images` 提交；到达的图片 `probe` 后记尺寸，尺寸变了就要重排。**重排节流**（`throttle.rs`）：两次重排之间至少隔 100 毫秒，**从上一次重排结束算起**（不是开始），所以一次重排很慢时消息循环也有空；没到时间就设一个定时器。**滚动锚定**（`interact::PageIndex::anchor`/`node_y`）：重排前找"离视口顶边最近且至少有一部分在下面"的第一段文字或图片，记下它的节点和离顶边的距离，重排后按同一节点的新位置把滚动位置调回去；滚动位置为 0 时不锚定。**每页最多 24 次由图片引起的重排**（`MAX_IMAGE_RELAYOUTS`，缩放/改大小时清零）：一个 `srcset` 选择取决于上一次选择造成的宽度的页面会无限排下去，这是防线；之后学到的尺寸留给下一次窗口要求的重排。
5. **图片字节被缓存挤掉的**：`ImageStore::prepare` 报出 `missing`，窗口用新加的 `PageResources::forget_images` 让它们能再请求（1c-2a 留的洞）。
6. **停止**（Esc 或 ✕）：`cancel_all` + `PageResources::abandon_pending`（被取消的请求不会再报告，计数要清掉，不然"加载中"永远不结束）；还在等样式表的页面马上用现有的显示。
7. **错误页**（`errorpage.rs`）：内置 HTML（标题、说明、地址、提示、详细信息；地址和详情都转义），经 `parse_document` 交给我们自己的内核显示，和普通页面同一条路；网络错误各种、被拒绝的地址、不支持的类型、地址解析失败、HTTP 4xx/5xx 没有正文（有正文的显示网站的内容）、内部错误（"这个网页触发了轻览的错误"）。错误页进历史（后退能回去）。错误页自己出错就显示空白，不再递归。
8. **`catch_unwind`**：`Browser::event` 整个包起来；panic → `internal_error`：取消加载、丢掉页面、显示"内部错误"页（消息只截 300 字符放在详细信息里）。栈溢出接不住，靠 8 MB 栈和各处递归上限。**主线程跑解析/样式/排版/绘制**（8 MB 栈）；另起的线程只有预热（`stack_size(8 MB)`）和 `Loader` 的请求线程（1 MB，只做收发，不递归；引擎里的，没动）。

### 交互细节

- **点链接**：`PageIndex::link_at` 遍历有行的块，取最后（最上）命中的文字或原子行内（图片）片段，沿 DOM 往上找带 `href` 的 `<a>`；**不考虑 `overflow` 裁剪**（被裁掉的链接也能点到，已知缺口）。`href` 按 `PageResources::base()`（含 `<base>`）解析。左键点 = 打开；Ctrl、Shift、中键、`target=_blank`（只看 `<a>`）= 新窗口（先过一遍 `policy::plan`，`mailto:` 在本窗口弹确认）。`#片段`链接且目标页就是当前页：不重新下载，按 HTML 7.4.2.3.3 找（先 `id`，再 `<a name>`；原样和百分号解码各试一次），`top`/空 = 页面顶部，进历史。
- **悬停**：每次鼠标移动做一次命中测试（线性，约 0.3 毫秒），变了就换手形光标、状态栏显示解析后的地址；先排第一屏的临时排版没有索引，所以那期间不命中（界面线程忙着，本来也收不到消息）。
- **滚轮**：`SPI_GETWHEELSCROLLLINES` 行 × 每行 100/3 CSS 像素（Chrome 默认 3 行 = 100 像素）；"一页"（`WHEEL_PAGESCROLL`）= 视口高的 87.5%；方向键一次 40 像素；PageUp/Down、空格 87.5%；Home/End；Ctrl+滚轮不缩放（第 1 层没有）。滚动位置取整到设备像素；每帧整个视口重画（1c-2a 实测够快，"平移旧画面"没做）。
- **前进后退**：`History`（最多 200 条，只存地址和离开时的滚动位置）；回去时 `use_cache: true`，页面和子资源都先问加载器的响应缓存（16 MB LRU），没有的才联网。跳转（重定向）后用最终地址更新历史条目。在同一个文档里的前进后退（只有 `#片段` 不同）不重新加载，直接滚到保存的位置。
- **标题**："页面标题 - 轻览"，没有标题时只写"轻览"（英文 "Qinglan"）。语言取 `GetUserPreferredUILanguages` 的第一项，以 `zh` 开头用中文。命令行给了地址就把焦点放到页面上（键盘能直接滚动），没给就在地址栏。

### `--bench` 怎么量（`crates/qinglan/src/bench.rs`）

- 打开真窗口（页面区 = `width × dpr` × `height × dpr` 设备像素；窗口超出屏幕也照样建，`WM_GETMINMAXINFO` 放宽到 16384），**`--width/--height` 是 CSS 像素**（和 `render` 一致），`--dpr` 默认 1；这样 dpr 1.5 的 1920×1080 是 2880×1620 设备像素，和 1c-2a 的 `render --scroll-bench` 同一个工作量（比 150% 缩放的真 1920×1080 屏幕重）。窗口启动后先等预热线程结束再建平台、开始加载（真实使用时网络比预热慢，预热早做完；回放瞬间到，所以这里等它，顺序和真实一致）。
- **冷首屏时钟**：从"HTML 到了"（回放的响应交到界面线程，解析之前）开始，到首屏像素贴到窗口、`GdiFlush` 返回为止。我们没有流式解析器，所以解析在 HTML 到了之后才做、被计入；等 `<head>` 样式表的时间是网络的时间，不计。`first-screen-ms` = 这段时间里界面线程忙于这个页面的时间之和（每个事件处理的时长相加：解析、发样式表请求、处理样式表到达、样式、排版、显示列表、光栅、贴窗口）；`first-screen-wall-ms` = 同一段的墙钟时间（多了回放里 26 张样式表的线程和读文件的等待），是上界。两个都打印，差别在无负载时 < 2 毫秒。`full-layout-ms`/`-wall-ms` = 同样的时钟到第一次整页排版结束（图片还没有）。
- **滚动**：所有图片先全部请求（`all = true`，和 1c-2a 的 `--full` 一样）并等重排稳定（没有在途请求、没有待办重排）；然后从头到尾每帧 100 CSS 像素，每帧由一条投递给自己的消息驱动（消息循环照常转），**每帧时间 = 光栅 + `SetDIBitsToDevice` + `GdiFlush`**，解码/丢弃图片的时间单独记（`scroll-decode-ms`，不计入帧时间）。到底（位置按设备像素取整后和最大滚动位置比，取整后的值比较，否则取整往下舍的底部永远到不了——这个错在 dpr 1.5 时让第一次 bench 无限滚下去，已修；另外加了帧数上限和 120 秒总期限，因为不停投递的消息会饿死低优先级的 `WM_TIMER`）后打印并退出。
- 内存：`K32GetProcessMemoryInfo`，`PrivateUsage`（滚到底后）和 `PeakPagefileUsage`（峰值）。启动时间：`GetProcessTimes` 创建时间到"窗口出来、地址栏可输入"（`Ready` 事件：窗口显示、焦点在地址栏、第一批消息处理完），用 `GetSystemTimeAsFileTime` 对表。
- 输出每行 `key: value`（`startup-ms`、`first-screen-ms`、`first-screen-wall-ms`、`full-layout-ms`、`full-layout-wall-ms`、`relayouts`、`scroll-frames`、`scroll-median-ms`、`scroll-p95-ms`、`scroll-max-ms`、`scroll-decode-ms`、`peak-private-bytes`、`private-bytes-after-scroll`，出错时加 `bench-error`，退出码 1）。`qinglan-cli bench <网址> --replay <目录>` 用同样的键和同样的时钟规则（没有窗口，没有贴窗口），另外打印各阶段（`parse-ms`、`style-ms`、`first-layout-ms`、`display-list-ms`、`raster-ms`）和三个时间点的内存。

### 验收数据（1c-2b，release，插电 `BatteryStatus = 2`，Intel i7-8850H，显示器 125%）

- **测量条件**：`tests/corpus/local/zhwiki-rec`，1920×1080 CSS 像素，每次新进程。**下面两组数字都不是干净环境**：另一个会话从 23:33 起一直在跑 qingpdf 的 `cargo test --release`（PID 2032，反复启动 `qpdf.exe`），CPU 占用在 10–20% 之间起伏、有突发。出现这个负载之前只量到 2 次（dpr 1，同一个二进制的前一版，改动只在滚动结束条件）：首屏 105.8、109.0 毫秒，整页排完 208、212 毫秒，滚动中位数 5.0 / 6.3 毫秒（p95 6.3 / 6.3），启动 78.8 / 66.2 毫秒，滚到底后私有内存 83.3 / 83.8 MB；`qinglan-cli bench` 同一时段：首屏 97.6 / 98.6，滚动 4.0 / 4.0（p95 4.8 / 5.0）。
- **带负载的 5 + 3 次**（`qinglan.exe --bench`，每次新进程，上面说的负载一直在）：dpr 1 五次：首屏（忙时间）250.8 / 248.0 / 303.3 / 214.1 / 227.1，**中位数 248.0 毫秒**（墙钟只多 0.5–1.3 毫秒）；整页排完 466.1 / 505.1 / 505.7 / 414.6 / 456.1，中位数 466.1 毫秒；滚动 711 帧，帧中位数 16.9 / 14.1 / 14.0 / 13.6 / 13.9，**中位数 14.0 毫秒**，p95 27.2 / 19.0 / 19.1 / 19.0 / 19.1，**中位数 19.1 毫秒**（最大 23–61）；启动 172.9 / 128.5 / 137.5 / 137.2 / 177.5，**中位数 137.5 毫秒**；滚到底后私有内存 88.1 / 84.6 / 83.2 / 84.4 / 83.5 MB，**中位数 84.4 MB**（峰值 = 同一个数）；重排 3 次。dpr 1.5（窗口 2880×1620 设备像素）三次：首屏 256.2 / 256.1 / 255.6，**中位数 256.1 毫秒**；整页排完 461.4 毫秒（中位数）；帧中位数 27.0 / 27.0 / 23.1，**中位数 27.0 毫秒**，p95 42.3 / 36.0 / 34.4，**中位数 36.0 毫秒**；启动 165.5 / 137.5 / 164.7，**中位数 164.7 毫秒**；私有内存 98.5 / 95.1 / 94.2 MB，**中位数 95.1 MB**。后来又量了一轮（负载更重：首屏 288–438 毫秒、帧中位数 14.7–17.2 毫秒），没跑完就停了，不采用。
- **和目标的关系**：干净环境下的两次（dpr 1）首屏 ≤ 200 毫秒、滚动 ≤ 16 / ≤ 33 毫秒、启动 ≤ 0.3 秒都达标；带负载时首屏（248 / 256 毫秒）超目标，dpr 1.5 的帧中位数（27 毫秒）超目标，其余仍达标。**这些超标是负载造成还是实现造成，这次没能分开——需要在空闲机器上重量一遍 5 + 3 次（1c-3 的验收测量）。** 私有内存不管负载都超（见下）。
- **体积**：release `qinglan.exe` 1 971 200 字节（1.88 MiB）；**超过 1.5 MB 目标**。试了 `opt-level = "s"`：1 575 424 字节，还是超，而且慢（带负载下首屏 580 对 417 毫秒、滚动中位数 35.8 对 27.1 毫秒），所以没改 profile。`qinglan-cli.exe` 1 966 080 字节，说明体积主要在引擎和 DirectWrite/WIC/WinHTTP 的声明代码里，要降到 1.5 MB 需要专门一轮（候选：去掉没用到的 Unicode/实体表重复、`#[inline]` 过多的排版函数、`opt-level` 只对引擎里的冷路径降级）。
- **依赖的 DLL**（读 PE 导入表）：`qinglan.exe`：`DWrite.dll`、`GDI32.dll`、`KERNEL32.dll`、`SHELL32.dll`、`SHLWAPI.dll`、`USER32.dll`、`WINHTTP.dll`、`api-ms-win-core-synch-l1-2-0.dll`、`bcryptprimitives.dll`、`ntdll.dll`、`ole32.dll`，都是系统 DLL（WIC 经 COM 由 `ole32` 加载 `windowscodecs.dll`，静态链接了 C 运行库，没有 vcruntime）。栈保留 8 388 608 字节（两个 exe）。
- **内存没达标**：滚到底后私有内存约 83–88 MB（目标 ≤ 50 MB），dpr 1.5 约 94–98 MB。`qinglan-cli bench` 的分段：解析后 16 MB，首屏后 44 MB，整页排版和全部图片后 73 MB，滚完 82 MB。大头是 DOM、样式树、整页排版结果和显示列表、字形遮罩缓存、响应缓存（5 MB 的录制全在里面）和 8–18 MB 的帧缓冲；要降到 50 MB 需要专门一轮（候选：整页排版后丢掉不用的中间结构、样式树瘦身、显示列表按需生成），不在 1c-2b 范围。
- **手工在窗口里验证过**（`PrintWindow` 截图）：中文维基首屏（工具栏、地址栏、原生滚动条、状态栏都对，字和图正常）；PageDown ×2 滚动、Home 回顶；悬停链接状态栏显示解析后的地址；点 `#歷史` 目录项滚到该节、地址栏带片段、后退可用；Alt+← 回到顶部且前进可用；地址栏输入 `ftp://…` 显示错误页（标题"This page can't be opened - Qinglan"，说明、地址、提示、详情齐全）；输入 `rust lang` 变成搜索地址；`--bench` 在 dpr 1 和 1.5 下跑完退出码 0。

### 没做 / 留给 1c-3

- `#片段`的第一屏要"排到目标再多一屏"（见上，偏离）；`overflow` 裁剪里的链接命中；`WM_DPICHANGED` 在多显示器上的实测；Ctrl+滚轮缩放（第 1 层没有）；表单控件交互（第 1 层没有）；窗口位置/大小不记忆；工具栏按钮没有提示文字。
- 内存和体积目标没达标（见上）；真实网页库的中位数/最大内存是 1c-3 的事。
- 开放问题：①内存 50 MB 目标是否按"滚到底、所有图片都请求过"算（这里是）；②冷首屏时钟在没有流式解析器时的定义（这里取"忙的时间之和"，同时给墙钟）；③dpr 1.5 的 1920×1080 CSS 像素是否就是验收说的"150%"（这里是，比物理 1920×1080 屏幕的 150% 重 2.25 倍）。

## 第 1 层 · 1c-2c 内存和体积（2026-10-16）

### 怎么量

- `qinglan.exe --bench ... --mem` / `qinglan-cli bench ... --mem`：开计数分配器（`qinglan-win/src/sys/alloc.rs`，`System` 的包装，`unsafe` 只在 `sys/`，没开时每次分配多一次 relaxed 读）。不带 `--mem` 的 bench 不计数，时间数字用它的。带 `--mem` 时按"把它丢掉、看活堆缩了多少"给出各部分字节数（`mem-*`，`-blocks` 是块数），再加 DOM 内部拆分、布局各数组、Windows 堆的 `allocated/committed`（`HeapSummary`）、各阶段 `PrivateUsage`、全部丢完后的 `PrivateUsage`。
- 结论：私有内存 ≈ 堆里 committed（比我们要的多 13–14 MB：每块约 16 字节头 + 释放后不还给系统的空闲块 + 峰值）+ 约 3 MB 非堆。`HeapCompact` 试过，没有效果，没留。

### 改了什么（zhwiki-rec，1920×1080，滚到底，私有内存 83.6 → 49.1 MB，十进制 MB）

- 整页布局 23.3 → 6.9 MB：`LayoutBox` 把少见字段（`replaced/control/synth/ordinal/image`）放进 `Option<Box<Rare>>`（160 → 约 72 字节）；布局完成时对保留的数组 `shrink_to_fit`；显示列表建好后 `LayoutResult::release_text_data()` 丢掉 `glyphs`/`texts`（只有建显示列表和 dump 读它们；命中测试和滚动锚定用盒子/行/碎片）。
- 帧缓冲 8.3 → 2.1 MB：窗口按条带画（`render_band`，条带 ≤ 2 MiB，1080p 四条）再逐条 `SetDIBitsToDevice`（`blit_at`）；像素和整帧逐像素相同（`paint.rs` 里有测试）。窗口里本来没有第二块全尺寸缓冲，所以没有"直接画进 DIB section"可省的；DIB section 的内存不算在私有字节里，那样只是挪出统计，没做。条带更小（0.5 MiB，16 条）窗口里每帧慢约 10%，所以取 2 MiB。
- DOM+样式表 10.2 → 7.5 MB：标签的属性数组按实际个数分配（原来每个元素留 4 个的位置）；解析完 `nodes.shrink_to_fit()`。
- 显示列表 5.0 → 4.1 MB：建完 `shrink_to_fit`。
- 响应缓存：预算 16 → 8 MB，**单个响应 > 1 MB 不存**（zhwiki 的页面本身 2.4 MB，比其余加起来还大；后退到这么大的页面要重新取）；加载器只留 `Content-Type`、`Cache-Control` 两个头（引擎只读这两个；真实服务器发二三十个头，每个两块堆）。zhwiki 的样式表和图片共 2.6 MB。
- 缓存预算：字形遮罩 8 → 1.5 MB，排版整形 8 → 1 MB，词表上限 6 万 → 3 万词，解码图片 24 → 12 MB（zhwiki 滚完峰值 3.8 MB）。
- 没改：响应缓存里的图片压缩字节（回滚要用）、DOM 属性值的单独堆块（8.98 万块，换成共用池要动 DOM 接口）。

### 体积

- 没达标：release `qinglan.exe` 1 971 200 → 2 148 864 字节（比 1.5 MiB 多 576 KB）。这一轮加的东西（分配器包装、`--mem` 的统计代码、条带绘制、`Rare` 访问器）让代码多了约 177 KB，没来得及拿回来。
- 试过：`qinglan-win` 用 "s"、引擎用 opt-level 2：2 096 640（只省 51 KB），窗口里滚动中位数慢 8%，没采用。非 LTO 构建的链接映射（/MAP，v0 名字，脚本按模块累加）：.text 1.6 MB，其中 `qinglan_engine` 650 KB（`html::tokenizer::step` 42 KB、`layout::inline` 44、`css::parse_prop` 48、`cascade` 42、`selector` 34）、core 198（`drop_glue` 77）、std 112（`process::spawn` 24、mpmc 21）、hashbrown 90、alloc 97；.rdata 约 370 KB。要到 1.5 MiB 得把引擎里冷的部分（net、encoding、url、css 的属性解析）拆成单独的 crate 用 "s"，或者去掉 `--mem` 统计代码（用 feature 关掉），这一轮没做。

## 第 1 层 · 热代码和冷代码、开发版内存统计（2026-10-16）

背景：`PLAN.md` 已定第 6 条（体积）：保留崩溃保护；热点代码按速度优化，其余按体积优化；内存统计只放开发版；第 1 层 exe 目标 1.9 MiB（1 992 294 字节）。

### 做了什么

- 计数分配器（`CountingAlloc`）和 `--mem` 的分项统计（`qinglan-win` 的 `memstats`、`sys/alloc.rs`、两个 bench 里的拆分代码）放到 cargo feature `mem-stats` 后面，默认关；`qinglan`、`qinglan-app`、`qinglan-cli` 各有同名 feature 往下传。`#[global_allocator]` 只在开了 feature 时才有，发布版用系统分配器（以前每次分配多一次原子读）。不带 feature 的 `--mem` 在 `qinglan-cli` 里提示"忽略"，在 `qinglan.exe`（没有控制台）里不处理。`--bench` 里的私有字节（`GetProcessMemoryInfo`）发布版照样有。这一项单独就省了 172 032 字节（2 148 864 → 1 976 832）。
- 按包设 `opt-level`（`Cargo.toml` 的 `[profile.release.package.*]`）：`qinglan-engine`、`qinglan-app`、`qinglan`（薄壳）、`qinglan-cli` 用 3，`qinglan-win` 用 "z"；`lto = true`（整体 LTO）、`codegen-units = 1`、`panic` 仍是 unwind，不变。
- `qinglan` 拆成两个包：`qinglan-app`（库，原来 `qinglan/src` 里除 `main` 以外的全部，`pub fn run()` 是入口）和 `qinglan`（只剩 `main.rs`、`build.rs`、`/STACK`、`windows_subsystem`、开了 feature 时的全局分配器，调用 `qinglan_app::run()`）。

### 实测结论（为什么是这样分）

1. **整体 LTO 下按包设 opt-level 有效**。同一份代码，引擎 3、其余 "s"：1 955 328（vs 全 3 的 2 148 864）；全 "s"：1 620 480；全 "z"：1 408 000。LTO 前每个包按自己的级别生成代码，函数带 `optsize`/`minsize` 属性，LTO 后保留。不需要换成 thin LTO 或关 LTO。
2. **但最后链接的那个包（二进制包）的 opt-level 决定 LTO 阶段的优化级别**。把 `qinglan`/`qinglan-cli` 设成 "s" 或 "z"，引擎里的热点代码（光栅化）明显变慢：`qinglan-cli bench` 滚动中位数 4.0 → 4.7–4.8 ms（+17–20%），p95 +18%，解析/样式/首屏几乎不变；设成 3 就回到 4.0。win 设 "s" 或 3 对这个没有帮助（单独试过）。所以二进制包必须留在 3，里面的代码要少：这就是把 `qinglan` 拆成薄壳 + `qinglan-app` 的原因（薄壳设 3，`qinglan-app` 不用跟着设 3 也能保住引擎速度）。
3. **`qinglan-app` 里有窗口的绘制和滚动路径**（`draw_bands`、`page.rs` 的图片准备、条带绘制），设成 "z" 时窗口里 `qinglan.exe --bench` 的滚动中位数 4.8 → 5.3 ms（+10%，安静机器、插电、10 轮交替、范围不重叠），所以保留 3。`qinglan-win` 里的 DirectWrite 粘合、编码、WinHTTP 用 "z"，实测不影响速度（见下）。
4. 拆出冷 crate 试过两次（`qinglan-base`：`url`、`lru`、`fx` 哈希、`net` 的策略/加载器/缓存/回放传输，设 "z"）：体积再省约 50 KB（1 864 704 → 1 814 528，那时 app 是 "z"），但 4 轮对比里 cli 的整页布局中位数 +6–8%、最小值 +6%，超过 5% 的上限；原因没查清（布局和绘制路径里没有这些代码，引擎各模块大小在拆前后几乎一样），没采用，代码没留。以后要拆的话，先单独量布局。`css` 的属性值解析（`parse_prop` 约 69 KB、`color`/`values`/`media`/`syntax` 等）也评估过没拆：它和 `types`、`computed` 互相引用（`types` 用 `color`/`values`/`syntax`），拆出去后热代码要跨 "z" 包调用小函数，LLVM 不会内联 `minsize` 函数。`encoding` 只有 8.6 KB 且依赖 `Platform` 特征，没拆。

### 体积（release，字节）

| 步骤 | `qinglan.exe` |
| --- | --- |
| 起点（c4e52f5） | 2 148 864 |
| 关掉 `mem-stats` | 1 976 832 |
| 另加 win/app/二进制包 "z"（速度不合格：滚动 +17–20%） | 1 779 712 |
| 薄壳 3 + `qinglan-app` "z" + win "z"（窗口滚动 +10%） | 1 864 704 |
| **采用：薄壳 3 + `qinglan-app` 3 + win "z"** | **1 925 120**（目标 1 992 294，余 67 174，3.4%） |

`qinglan-cli.exe` 2 134 528 → 1 914 880。导入的 DLL 只有系统自带的：kernel32、ntdll、user32、gdi32、shell32、shlwapi、ole32、dwrite、winhttp、bcryptprimitives、api-ms-win-core-synch。

链接映射（`cargo rustc --release -p qinglan -- -C link-arg=/MAP:...`，整体 LTO 下符号都在静态符号段，按 v0 名字累加）：.text 约 1.36 MB，其中 `qinglan_engine` 652 KB（css 327、layout 136、html 97、url 34、net 25、paint 21、encoding 9）、core 168、std 146、`qinglan`（app）107、win 66、hashbrown 52、alloc 51；.rdata 约 200 KB，.xdata + .pdata 约 194 KB（unwind 表，保留崩溃保护的代价）。其余零碎：`std::process::Command`（开新窗口用）约 28 KB，换成直接调 `CreateProcessW` 可以省，没做（要加一段 unsafe 和命令行转义）。

### 速度和内存（插电，BatteryStatus = 2，旧二进制和新二进制交替跑）

- 窗口 `qinglan.exe --bench`（zhwiki-rec，1920×1080，10 轮，安静机器，中位数 旧 → 新）：首屏 104.3 → 105.0 ms，整页布局 206.4 → 214.1（+3.7%），滚动中位数 4.9 → 4.8，p95 6.3 → 6.5。
- `qinglan-cli bench`（同页面，结构相同的二进制，二进制包 3 + win "z"）：首屏 98.0 → 97.3，整页布局 196.1 → 199.6（+1.8%），滚动中位数 4.0 → 4.0，p95 5.0 → 5.0。
- `cargo test --release -p qinglan-engine --test perf`（4 轮交替，另一个会话在编译，5 个 cargo/rustc 进程）：解析+样式合计 65.6 → 66.2 ms（+0.9%）。引擎包没变，级别也没变。
- 内存：滚到底后私有字节 48.5–48.9 MB（十进制 MB），目标 ≤ 50 MB；旧二进制 47.6–49.7。峰值约 54 MB。
- 噪声：另一个会话的 `cargo test`（qingpdf）经常占 CPU，负载高的时候数字能差 2–3 倍，甚至同一个二进制差 10%；上面的窗口数字是等它跑完、没有 cargo/rustc 进程时量的。

## 第 1 层 · 1c-3 真实网页库、恶意网页、验收数据（2026-10-16）

数据都在 `tests/corpus/RESULTS.md`；网址和日期在 `tests/corpus/PAGES.md`；跑法在 `tools/corpus_run.py`（`record`、`render`、`edge`、`compose`、`check`、`bench`、`winbench`、`pages` 各一步；测试用的脚本，不是产品代码）。

### 选择

- **Edge 对照怎么跑**：任务单的 `--blink-settings=scriptEnabled=false` 在这台机器的 Edge 155 上让无头模式不出截图（进程立刻退出，没有文件），所以改成：每次在临时目录里建一个新的空 profile，写 `Default/Preferences`（`profile.default_content_setting_values.javascript = 2`）关 JS；验证过脚本不执行、`<noscript>` 显示。不碰用户的 profile。Edge 写不进仓库目录，截图先写到临时目录再拷。Edge 起的进程按命令行里带的 profile 目录名找，没退干净的才杀。
- **“文字完整、顺序对”怎么判**：肉眼不可能逐字看 49 页，所以用脚本：DOM 里显示的文字（去 script/style/template/head 和 `display:none` 子树，靠 `style` 输出和 `dom` 输出逐元素对齐）和 `layout` 输出里的文字，12 字符一块，在 4000 字符窗口内按序查找（窗口是为了页面里重复的标题不把指针拖走）。双向都查：多出的、丢的。已知的假差：`text-transform`、坏图的 `alt` 文字。
- **Stack Overflow**：对我们的 User-Agent 回 Cloudflare 的 403 挑战页。不换 UA 去绕过（机器人检测不绕），录的就是挑战页；另录 Discourse 和 V2EX 的主题页当问答页。
- **Big5**：试了约 80 个站，只有 cs.nthu.edu.tw/~king/ 还在用 Big5，一页，3.8 KB。
- **图片缓存**：图片响应单个上限 = 缓存预算的一半（4 MB），其他仍是 1 MB（1c-2c 的决定留着，针对 2.4 MB 的维基页面）。原因：画图从缓存里取压缩字节解码，大于上限的图片永远画不出来，窗口还会无限重新下载（真实页面 xinhua 的 1.6 MB PNG 头图）。另外 `ImageStore` 对“字节不在缓存里”最多再请求 2 次（成功解码清零），之后标成失败，所以超过 4 MB 的图片不会死循环。
- **`qinglan-cli bench --max-frames N`**：滚动帧数上限，默认 20 000；恶意页面测试用 10。没有它，高 1e9 像素的页面会滚几个小时。输出多一行 `scroll-complete`。
- **恶意页面测试放在 `qinglan-cli/tests/hostile.rs`**：每个用例写成一份录制，在子进程里跑 `qinglan-cli bench`（解析、样式、排版、显示列表、光栅、滚 10 帧都走），子进程自己报 `PeakPagefileUsage`，所以峰值是页面自己的。用 `Mutex` 让用例一个一个跑（5 秒是单个页面的时间）。release 下按任务单的尺寸，debug 下尺寸缩到十分之一、只查内存（debug 太慢，时间不可比）；子进程有 60/300 秒的硬停。坏 UTF-8 用 2 MB（4 MB 196 MB、5 MB 198 MB，贴着 200 MB 上限，测试会不稳定；见 RESULTS 疑似问题 #8）。
- **1 GB 的 gzip 炸弹**：测试里现场生成（固定霍夫曼块，一个字面量 0 加 400 多万个“长度 258、距离 1”的匹配，13 位一个，压缩流 6.7 MB），不往仓库里放 1 MB 的二进制；trailer 的 CRC 是假的，因为传输在 16 MB 上限处就停了。
- **150% DPI 的滚动**：用 `--width 1280 --height 720 --dpr 1.5`（窗口 1920×1080 设备像素），和物理 1920×1080 屏幕 150% 缩放的工作量一致；1c-2b 用的 CSS 1920×1080 × 1.5 比它重 2.25 倍，不是验收要的。

### 结果摘要

- 真实网页 49 页：全部能打开，文字完整顺序对，方块 0、乱码 0；一个 bug 已修（图片缓存）；其余差别是第 2 层特性，加疑似 bug 清单（RESULTS 第 1 节）。
- 恶意网页：18 项在 release 下都 ≤ 5 秒（最慢 2.1 秒）、峰值 ≤ 200 MB（最大 css 10 万行 171 MB、坏 UTF-8 179 MB），网络四项同；见 RESULTS 第 2 节。
- 验收数字（release，插电）：exe 1 927 168 字节；启动 58 ms；冷首屏中位数 102.9 ms（整页排完 212.8 ms）；滚动 100%：4.9 / 7.0 ms，150%：5.6 / 7.4 ms；滚完私有内存 48.9 MB 中位数（47.5–51.8，第一次超了）。

## 第 1 层 · 1c 审查修复（2026-10-16，提交 1：安全）

### S1 页面不能让浏览器打开 `file:`

- 问题：`navigate` 和 `open_in_new_window` 用 `policy::plan(url, Document, None)`，`None` 的意思是“用户自己打字的”；点击链接（含 Ctrl/Shift/中键、`target=_blank`）和 `<base href="file:///C:/">` 解析出来的页面链接也走这条，于是网页能链到本机文件。
- 修法：应用层加 `Origin { User, Page }`（`app.rs`）。打字、命令行、前进后退、刷新是 `User`（`None`）；点击链接、新窗口都是 `Page`，把当前页面的地址作为 `page` 传给 `plan`，`file:` 一律 `Refusal::LocalFile`（状态栏/错误页提示）。新窗口是新进程，参数当作打字的，所以**在父进程里先过一遍 `Page` 的规则**才启动。判断集中在 `plan_navigation`（纯函数，有单元测试）。`<base href=file:>` 不改变这一点：来源是页面地址，不是 base。
- 保持原样：本地 `file:` 页面里的链接也不跟随 `file:`（已有测试 `links to file: are not followed`），本地页面可以载入本地子资源。
- 纵深：`net/file.rs::url_to_path` 不管谁要求，一律只认本机普通文件：主机不是空/`localhost` 的 `file:`（网络共享）、解码后（`\` 当 `/`）以 `//` 开头的路径（UNC、`\?\`、`\.\`）、任何一段是 DOS 设备名（`CON`、`NUL`、`COM1`…，带不带扩展名；打开它们会让工作线程永远卡住）都拒绝。`policy::plan` 里这种情况是新的 `Refusal::NotLocalFile`。
- 没有 `<meta http-equiv=refresh>` 或表单提交，所以“无表单的重定向”只有 HTTP 重定向：传输层本来就只跟 http/https（`BadRedirect`）。iframe 只画占位框，没有链接。

### S2 `mailto:`

页面给的 `mailto:` 地址原样交给 `ShellExecuteW`，邮件程序注册表里的命令行是 `"…" "%1"` 一类，地址里的 `"`、空格、`\` 可以带出额外参数。现在 `policy::sanitize_mailto` 重建地址：只放行字母数字和 `- . _ ~ ! $ ' ( ) * + , ; : @ ? = &`（RFC 6068 的结构字符），合法的 `%XX` 保留，其他字节（空格、`"`、`\`、`/`、不合法的 `%`、控制字符、非 ASCII）一律 `%XX`；去掉 `#片段`；最长 2048 字节，在两个单元之间截断（不会截在 `%XX` 中间）。确认对话框显示的就是交给系统的这个字符串。

### S3 `Loader::cancel_all` 立刻放出名额

之前被取消的请求要等工作线程返回才释放 `in_flight`/`per_host`，沉默的服务器能让 16 个名额占 60 秒，新页面的请求排不上。现在 `cancel_all` 把两个计数清零，旧一代（`generation`）的结果到达时不再减计数（`finish` 先比代数）。工作线程本身靠 S4 的看门狗关闭请求句柄来结束。

### S4 看门狗：总时限和取消都靠关句柄

WinHTTP 的超时是分步的（解析、连接、发送、接收各一个），沉默服务器的总耗时能超出 4 秒以上，被阻塞的线程也看不到取消标志。`qinglan-win/src/net.rs` 里有一个 `Watchdog` 线程（第一个请求时才起，没有请求就睡在条件变量上）：每个请求在 `Session::get` 创建请求句柄之后、发送之前登记（总截止时间 + 取消标志）；到点或取消标志被设置（每 25 ms 看一次，只在有请求时）就关掉请求句柄，阻塞的调用立刻返回（`12017`），再按“取消 / 超时”归类（`error_for`）。

句柄的安全：请求句柄放在 `Shared(Mutex<Option<Handle>>)` 里，使用线程每次调用前通过 `raw()` 从槽里取，关掉的槽返回错误，所以不会拿已经关闭的句柄开始新调用；谁把句柄从槽里取出来谁关，只关一次（`HttpRequest::drop` 也走这条）；已经在阻塞的调用正是 WinHTTP 支持的情形（句柄关闭时调用失败，对象等调用返回才释放）。unsafe 仍只在 `sys/winhttp.rs`。测试：`silent_server_times_out`（1.5 s 总时限，断言 1.4–2.5 s）、`cancelling_aborts_a_blocked_request_quickly`、`cancel_all_frees_the_slots_and_aborts_the_blocked_workers`（3 个沉默服务器 18 个请求，16 个在跑；取消后名额为 0，新文档请求马上开始，句柄在 25 ms 级被关掉）。

## 第 1 层 · 1c 审查修复（2026-10-16，提交 2：真实网页问题）

### B1 混合内容的图片升级

https 页面上的 `http://` 图片改成 `https://`（同主机，端口不变，默认端口隐含）去取，失败就是坏图，不回退 http。只对图片：样式表等其他子资源仍然被 `Refusal::MixedContent` 拦住。升级发生在 `PageResources::request_images`，缓存键、去重键、`Referer` 用升级后的地址。

### B2 iframe 等占位框不遮挡

`ReplacedKind::Placeholder` 不再填灰色：只画 1 px 灰框和名字。`position: fixed` 的侧栏 iframe（mdBook）不会再盖住正文。

### B3 asahi 和 zol 的缺图

查清了，没有要加的功能：asahi 是 SVG（第 2 层）加 lazysizes 的 `data-src`（要 JavaScript），zol 是混合内容（B1）加 `data-lazyload-src`（要 JavaScript）。没有 `<picture>`/`srcset` 的问题。不做 `data-src` 一类的猜测：那是各站自己的脚本约定，不是 HTML 标准。

### B4 首屏时间：样式表解析

首屏慢在样式，不在排版停止线。三项改动，行为不变（49 页的 `style` 输出逐页相同）：

- `css/facts.rs`：先扫一遍文档，记下出现过的标签、id、class、属性名、属性（名, 值）的哈希。作者样式表里选择器的任何一个复合选择器要求文档里没有的东西，这条选择器就丢掉（全部丢光的规则不解析声明）。只判断“确定不可能”：伪类、`:not`/`:is`、`*`、值的前缀/子串/不区分大小写比较一律保留；怪异模式不判断 id 和 class（那里不区分大小写）；UA 样式表不过滤。哈希碰撞只会多留一条规则。
- 当前视口下不成立的 `@media` 块在转换时就跳过，不解析里面的规则（以前是解析完、建索引时才丢）。
- `parse_cvs_many`：作者样式表和其中的 `@import` 之前的分词（文本到组件值）在最多 4 个线程上并行，总文本不到 256 KB 时不用线程；每个线程 8 MB 栈，和主线程一样。样式表的获取、解析成规则、建索引和 `@import` 仍然按文档顺序，在调用线程上，所以结果和顺序不变。
- `@import` 进来的样式表仍然是按需在建索引时解析的（也过滤，但不并行）。
- 工具：`qinglan-cli style --time` 多输出各阶段时间（`StyleTimings`、`CascadeTimings`）。

### 没做

- B5、B6 在提交 3 做了，见下一节。

## 第 1 层 · 1c 审查修复（2026-10-16，提交 3：内存）

测量：插电（`BatteryStatus = 2`），没有 cargo/rustc 在跑，别的是 Chrome、Claude 桌面版、Defender（`MsMpEng`）。`mem-stats` 版在 `target/mem` 里单独构建，不和发布版互相覆盖。

### B5 坏 UTF-8 的内存（4 MB 垃圾，峰值 207 → 106 MB，1.6 → 1.5 秒）

量出来的结构（`--mem`）：峰值出在 `prepare` 里，不在显示列表：整段（100 万字符）的字形数组（平台返回一份、拷进 `Layouter.glyphs` 一份）、平台整形的临时数组（每字节约 10 字节）、`Vec` 倍增、每个簇一个 `Unit`（40 字节）同时活着；整形之后文字在结果里还留着白空格处理时长出来的 16 MB 容量（3 MB 的内容）。分步（`qinglan-cli bench` 的私有字节峰值）：

| 改动 | 峰值 |
| --- | --- |
| 起点 | 203 MB |
| 长文字按 `SHAPE_CHUNK` = 64 KiB 分段交给平台（`shape_item`/`shape_piece`，在空格或换行后切，否则在不会和前一个字符粘连的字符边界切，切不到就硬切）；字形和单元数组对长文字一次 `reserve`；整形后给过大的文字 `shrink_to_fit`；`line_breaks` 的数组按字符数而不是字节数留容量 | 168 MB |
| 字形数组 `reserve` 多留 1/16（字形数比字符数多 0.8%，刚好够就会倍增到 200 万） | 130 MB |
| 布局和显示列表里的字形改用 `PlacedGlyph`（16 字节：`font`/`glyph` 各 16 位，没有簇范围；原 `Glyph` 28 字节），`PaintGlyph` 20 → 16 字节 | 117 MB |
| 平台词表上限 3 万 → 1.6 万词（垃圾文字里每个脚本段都是一个“词”，3 万条约 9 MB；zhwiki 约 7 千条，不受影响） | 106 MB |

- 分段只影响超过 64 KiB 的单个文字项；其余页面整形调用和以前逐字节相同。切口的判断（`joins_previous`）和平台自己切 8192 单元长段时用的是同一张表。
- `PlacedGlyph` 去掉了簇范围：`qinglan-cli layout` 的缺字框统计原来用它找字符，现在按字形在片段里的位置取字符（簇是一对一时一样，多字符簇前面有的话会错位；只影响这个调试输出）。字体和字形编号超过 65535 时落到 0（.notdef）；平台最多 512 个字体，OpenType 字形编号本来就是 16 位。
- 没做：压缩 `Unit`（40 字节，100 万个单元仍占 42 MB，是现在峰值里最大的一块；要动 `kind`/`first`/`last` 的 40 多处用法）；合并连续相同的 U+FFFD（会改页面内容，而且这个测试的随机垃圾里很少有长串）；降低 100 万字符的上限。
- 测试：`hostile.rs::bad_utf8` 恢复 4 MB（debug 仍是十分之一），发布版断言峰值 ≤ 120 MB（其余用例仍是 200 MB）。

### B6 zhwiki-rec 的内存余量

- 数据：构建后立刻新开 5 个 `qinglan.exe --bench` 进程，滚到底的私有字节 47.9、48.4、48.2、49.2、48.2 MB（十进制 MB），峰值 50.7–51.3 MB。最大 49.2 MB，余量不到 1 MB。
- “构建后第一个进程高 4 MB”这次没有重现：第一个进程是 47.9 MB。试过：(1) `qinglan-cli --mem` 的第一次和之后两次，活堆都是 45 515 372 字节，分阶段私有字节差不超过 0.4 MB；(2) 把 `qinglan.exe` 复制成新文件名当“第一次”跑，47.6 → 48.5，48.4 → 48.2，没有差；(3) 进程里加载的 DLL 都是系统的（没有杀软注入的模块）。所以不是我们代码里的东西，也不是新文件被扫描；上一轮看到的 +4 MB 更可能是当时机器上 cargo/Defender 还在收尾。没查出来，也没有因此改代码。
- 这轮真正降的：显示列表 4.14 → 3.58 MB（`PaintGlyph` 16 字节）、构建期间的字形临时数组（峰值），词表上限降低对 zhwiki 没有影响。比 B6 前的数据（48.1–49.4）没有明显变化，要稳稳压到 50 MB 以下，下一步是 DOM 属性值共用一个池（8.96 万个小块，每块堆头加取整约 24 字节，约 2 MB），要动 DOM 接口，这轮没做。

### 体积

`qinglan.exe` 1 985 536 → **1 924 608**（上限 1 992 294，余 67 686；目标 ≤ 1 965 000 达到）。开新窗口的 `std::process::Command` 换成 `qinglan-win/src/sys/process.rs` 里直接调 `CreateProcessW`（命令行在 `spawn.rs` 里用安全代码拼，按 C 运行库规则加引号，带测试；含 NUL 的参数拒绝）。S1 的规则没动：`spawn_window` 收到的地址和以前一样，来自已经检查过的 `url.href()`，这次只是少了一层库。代码里放 `unsafe` 的地方仍然只有 `sys/`。

### 速度（`qinglan-cli bench`，zhwiki-rec，1920×1080，旧、新交替 4 轮，中位数 旧 → 新）

首屏 100.4 → 99.5 ms，整页布局 203.3 → 200.1 ms，滚动中位数 4.0 → 4.0 ms。新二进制第 3 轮有一次被别的东西打断（113.0 / 267.8 ms），其余三轮是 98.6–100.1 / 198.5–201.2；没有回退超过 5%。


## 第 2 层 · 2-0 第 1 层收尾（2026-10-16）

### 1. 滚动卡顿

**大小已知的图片只重画。** 排版时在 `layout/replaced.rs` 的 `replaced_size_final`（`block.rs` 里盒子最终定尺寸的那一次调用）为每个图片盒子记一条 `ImageFit`：指定的宽、高（`auto` 就是没有）、`min/max-width/height`、`srcset` 选出的密度、排出来的内容尺寸；记在 `LayoutResult::fits`（按 DOM 节点）。图片头部到了，`Page::apply_image` 问 `ImageFit::keeps_size(像素宽, 像素高)`：用同一个纯函数 `solve`（从 `replaced_size` 里原样拆出来的 CSS2 10.3.2/10.6.2 那段）按图片的真实大小重算，宽必须相同、高相差不超过 1 px（`HEIGHT_SLACK` = 64 个布局单位），就只 `invalidate`，不重排。宽度没有指定的（`spec_w` 是 `None`）一律重排：盒子的宽度和包含它的盒子的固有宽度都跟着图片走。
- **为什么高度允许差 1 px**：`width`/`height` 属性是整数，MediaWiki 的 `height` 是按比例四舍五入的（250×180 的缩略图显示 180 宽，写 `height="130"`，真实比例给 129.6）；zhwiki-rec 里第一版严格相等的判断让 21 次整页重排都因为这 0.1–0.5 px 发生。整页重排约 200 毫秒，换来不到 1 px 的差别不值。代价：这个盒子保持属性给的高度，直到下一次整页排版（改窗口大小、真需要重排的图片）；全新排版会用图片的真实比例，所以和新开一次页面比，每张图最多差 1 px，不累积成误差（互相独立，正负都有）。
- **加载失败**：有盒子的图片（`fits` 里有）坏了，盒子和没加载时一样大，不重排；显示 alt 文字的图片（没有宽高、有 alt，`Special::Alt`，不进 `fits`）照旧重排。`Page::request_images` 里加载器直接拒绝的图片同样处理（`break_image`）。
- 引擎没有 `aspect-ratio` 属性（只有媒体查询里的），任务单说的"aspect-ratio"在这里就是 `width`/`height` 属性给出的比例。
- 没做的：宽度没指定的图片（老网站里的无尺寸图片）仍然整页重排。要不重排得做局部排版，第 2 层后面再说。

**其余重排：滚动停了再排，合并成一次。** `Throttle` 多了 `quiet`（150 毫秒）和 `scrolled(now)`：窗口里每次滚动输入（`scroll_to`，滚轮、滚动条、键盘都走它）和 bench 的每一步记下时间；`plan` = 距离上一次排版结束的 100 毫秒间隔和距离最后一次滚动输入的 150 毫秒，取较长的等；等的时候来的请求都并进这一次（`pending` 只是一个标志）。**没有设最长推迟时间**：拖着滚动条不放时，需要重排的图片到松手后 150 毫秒才排。`image_relayouts`（每页上限 24）原来数的是请求，合并以后请求很多，改成数真正做了的图片引起的整页排版（缩放/改窗口大小仍然清零）。

**`--bench` 像人那样滚。** 每格 100 CSS 像素（滚轮一格），10 格一组，格间用 16 毫秒的定时器（系统取整成 16 或 31 毫秒），组间停 200 毫秒；每格和窗口一样调 `relayout.scrolled` 和 `request_images`（图片懒加载：近了才请求，`all` 不再因为 bench 而打开），所以图片在滚的时候到；到底后等排版和图片落定（最多 5 秒）再收尾。一次 zhwiki-rec 约 40 秒（以前约 4 秒）。超时从 120 秒放到 240 秒。新开发选项 `--replay-delay <毫秒>`（要 `--replay`，0–10000）：回放的每个响应先睡这么久，再加最多一半、按网址哈希定的额外时间，让图片像真实网络那样在滚动中陆续到。**报告里的新行**：`scroll-max-frame-ms`（每步光栅 + 贴窗口 + `GdiFlush` 的最大值，和以前的"帧"同一个定义）；`scroll-max-ms`（滚动期间界面线程处理过的最长的一个事件：一步、一次图片到达、一次排版，含解码）；`scroll-max-ours-ms`（同上，减去那个事件里花在 GDI 贴窗口调用上的时间，即我们自己的代码占着界面线程的最长时间）；`scroll-max-present-ms`、`scroll-present-stalls`（贴窗口超过 50 毫秒的帧数）；`scroll-relayouts`（滚动期间的整页排版数和最长一次）；`scroll-slowest-frames`（最慢三帧的位置）。

**量到的现象（电源和负载见 RESULTS.md）**：
- 旧行为（每张到达的图都排，不等滚动）+ 新 bench：滚动期间 21 次整页排版（每次 130–180 毫秒），`scroll-max-ours-ms` 162–369 毫秒。新行为：滚动期间 0 次。
- **贴窗口的卡顿不是我们的。** 插电时那几次，每次 bench 有 6–23 帧在第一条带的 `SetDIBitsToDevice` 里卡 300–500 毫秒（`光栅` 不到 2 毫秒），多数是一组里的第二格，和图片到达、排版、滚动条重画（试过 `SetScrollInfo` 不重画）、条带大小（试过 0.5 MiB）都无关，新旧两个程序一样多；换成电池后同样的 bench 5 次里 0 次。判断是系统合成器（DWM/GPU）有时忙，GDI 的这次调用在内核里等。第 1 层验收复测里"滚动中偶尔有一帧 270–310 毫秒"十有八九也是这个（那个 bench 滚动期间没有重排）。我们能做的只是不在滚动中占用界面线程。
- **还剩的**：滚动期间最长事件 89–169 毫秒（电池、别的会话在编译时），是一步里 `prepare` 解码大图（在界面线程上）。解码已在 2-0 追加的第 5 节挪到后台线程；那个数字其实是 WIC 探头，不是解码，也在第 5 节处理了。

### 2. 启动偶尔慢（0.5 秒）

量法：`qinglan.exe --bench`（小页面 aozora-sjis，窗口出来、地址栏可输入的时间 `startup-ms`）新开进程 20 次；另量 5 个刚复制出来的新文件名、10 次旧 exe，和 20 次"在解析命令行时就退出"的进程（`--bogus`，还没建窗口，只量进程墙钟时间）。**这一组是在电池供电、另一个会话有 5 个 rustc/cargo 进程在跑时量的**（插电和空闲没等到；见 RESULTS.md）。
- 新 exe 20 次：137–267 毫秒占 17 次，中位数 195.5；最后三次 445、992、368（负载最重的时候）。刚复制的 5 个新文件：245–368，中位数 333（同一个文件反复跑的中位数 195）。旧 exe 10 次：149–257，中位数 171。`--bogus` 20 次：161–397 毫秒，中位数 205。
- **结论：不是我们启动路径上的东西。** ①连什么都没做、在解析命令行时就退出的进程也要 160–400 毫秒（它还没建窗口、没读字体），慢的时候也是 400；②刚复制出来的新文件第一次跑慢 100–170 毫秒（杀毒软件对新文件的扫描：这台机器有 Malwarebytes 和 Defender）；③最慢的几次都出现在别的进程把 CPU 占满的时候。条件：新生成的 exe 的第一两次运行、系统忙的时候。以前"5 次里 1 次 0.5 秒"符合这两种情况（构建后的第一个新进程；别的会话在编译）。我们的启动本身（建窗口到能输入）在空闲插电时 73–97 毫秒（第 1 层复测），没有改。

### 3. 内存余量：DOM 属性值共用一个池

`html/dom.rs`：DOM 里存的属性是 `StoredAttr`（命名空间、前缀、名字加 `u32` 的偏移和长度，20 字节），值放进 `Document::attr_pool`（一个 `String`，所有属性值首尾相接）；读值用 `Document::value(&StoredAttr)`，建树时 `Document::store_attrs` 把分词器给的 `Attribute`（还是 `String` 值，只在解析过程里存在）转进去，`token_attr` 给"活动格式化元素表"从 DOM 取回成标记。偏移超过 `u32` 的（16 MB 的页面上限之下不会发生）值存成空串。要读值的地方（`css/mod.rs`、`presentational.rs`、`selector.rs`、`facts.rs`、`cascade.rs`、`dump.rs`、`memstats.rs`）都改成 `doc.value(a)`。原来每个属性：40 字节的 `Attribute`（`String` 24 字节）加一个堆块（16 字节块头 + 取整）；现在 20 字节加池里的字节。
- 结果（zhwiki-rec，1920×1080，滚到底，新开进程 5 次，十进制 MB）：{45.44、44.12、44.74、44.83、44.91}，**最大 45.44**（目标 ≤ 46）；峰值 45.3–46.0。第 1 层复测是 47.3–48.1。**这一组也是在电池、别的会话编译时量的**（内存不受 CPU 速度影响，但图片到达的次序会）。重排从 3 次（以前整页 21 次）降到 0 次也省了峰值：重排期间新旧两份排版同时在内存里。
- 没开 `--features mem-stats` 重新拆分（目标已经达到）。

### 4. 体积、速度、测试

- `qinglan.exe` 1 924 608 → **1 954 304** 字节（上限 1 965 000，余 10 696）。加了 `ImageFit`、`solve` 拆分、池、bench 新指标。
- 速度（`qinglan-cli bench`，zhwiki-rec，1920×1080，旧、新交替 4 轮）：首屏 中位数 283.6 → 298.4（+5.2%），最小值 279.0 → 277.1（-0.7%）；整页布局 中位数 577.5 → 640.2（+10.9%），最小值 546.2 → 539.3（-1.3%）；滚动帧中位数 中位数 9.9 → 9.8（-1.5%），最小值 9.6 → 9.6（+0.0%）。电池供电、别的会话在编译，数字都慢一倍，中位数噪声很大，最小值才可比。
- 测试：`cargo test --workspace` 共 290 个通过、0 个失败；`cargo clippy --workspace --all-targets` 0 条警告。

### 5. 滚动里最长的界面线程事件：图片解码挪到后台，另外发现真正的元凶是 WIC 探头

上面第 1 节留下的 89–169 毫秒，当时记成"`prepare` 在界面线程解码大图"。这一步把解码挪到后台线程之后，`scroll-max-ours-ms` 一点没降（电池、别的会话在编译时 142–462 毫秒），于是在 `Browser::on_wake`、`Page::handle_update` 里临时加了分段计时（量完已删，没有留在代码里）。结果：滚动中最长的事件是 `wake`，里面是 `Page::apply_image` → `probe_response` → `WicDecoder::probe`，**每张图 60–460 毫秒，连 200 字节的图也一样**（空闲、插电时也是 100 毫秒上下）；解码一项在界面线程上只剩 11–13 毫秒（全程总和，`scroll-decode-ms`）。原因：`CreateDecoderFromStream` 要把系统里所有的解码器问一遍，WebP 的解码器是商店里的扩展包，每次都要激活；zhwiki-rec 一共 142 张图：71 张 WebP、55 张 JPEG、11 张 PNG、1 张 GIF、4 张 SVG。所以第 1 节那个数字其实一直是探头，不是解码。

**1. 解码池**（`qinglan-engine/src/paint/decode.rs`，`DecodePool`）。队列在界面线程一侧（和加载器一样），同时最多 `DECODE_WORKERS` = 2 个任务在跑，**每个任务一个线程，跑完就结束**（栈 2 MB，只是预留）。没有常驻线程、没有 `mpsc`、没有 `Condvar`：试过常驻 2 线程 + `Condvar` + `VecDeque`，`qinglan.exe` 比现在大 3.6 KB（最先的 `mpsc` 版更大），而一次解码几毫秒到几百毫秒，起线程几十微秒，不值。没开始的任务可以直接撤回（`cancel` / `cancel_all`，不用加锁）；线程之间只共用一个装结果的 `Mutex<Vec<Finished>>`，线程放完结果调加载器的同一个唤醒回调（`Loader::waker()` 新加的访问器），窗口收到 `Event::Wake`，在 `on_wake` 里 `poll`，同时给等着的任务腾出位置。所有线程都从 `net::loader::spawn_named`（一个不带泛型的函数）起，加载器和解码池共用，省掉一份 `thread::Builder::spawn` 的实例。线程里的解码包在 `catch_unwind` 里：坏图是失败的图，不是死掉的线程。

**2. 缓存怎么记账**（`ImageStore::prepare_async` / `accept`，`prepare` 同步版留着给命令行的 `render` / `bench` 用）：
- 走到池里的任务按目标尺寸记一笔 `reserved`（`宽 × 高 × 4 + 64`，和 `DecodedCache` 记法一样，是上限），**缓存已有字节 + 在途字节 + 这一张 ≤ 预算（`DECODED_CACHE_BUDGET`，代码里是 12 MB；上面 1c-2 记的 24 MB 是旧数字）才启动**。结果回来，先扣掉这一笔，再按实际大小入缓存。比预算还大的图不启动，和以前一样画不出来（以前是白白解码一遍）。
- 腾位置只丢"比等着的这张离视口更远"的位图（按 LRU），不丢更近的。如果谁都能丢谁，两张加起来放不下的图会互相踢出对方，永远在解码（测试里有）。放不下的等一个解码结束；`deferred()` 为真时，`on_decoded` 在结果入缓存后补一次重画，让它们补上。
- 候选按离视口的距离从近到远启动（同距离按文档顺序）。
- 没画出来的图什么也不画（页面本来的样子），不等。画到了才补：`Browser::on_decoded` 只重画这张图占的那几行（`Page::take_decoded` 算出设备像素的行范围，`draw_rows` 一次画一个条带，和滚动画整屏用同一套 `render_band`，`draw_bands` 现在就是 `draw_rows(0, 高)`），不通过 `WM_PAINT`、不重画整个窗口。
- **取消**：每次 `prepare_async` 先清空每个在途任务的"谁要它"名单，再按现在视口附近 1 屏内的图重新登记；没人要、而且还在队里没开始的，从队里撤回并退还预算；已经在跑的让它跑完，结果因为没人要被丢掉。离视口超过 4 屏的位图照旧由 `drop_far` 丢（`Page::prepare` 里先于启动新任务做，腾出的位置马上能用）。尺寸变了（缩放、重排）的图同理：旧尺寸的任务没人要，新尺寸另起。
- **导航之后才完成的解码被丢掉**：`ImageStore::accept` 只收自己登记过的 (地址, 宽, 高, 任务号)；新页面的 `ImageStore` 是新的，没有旧页面的任务，任务号在一个池里不重复，所以哪怕新旧页面要同一个地址同一个尺寸，旧结果也对不上。`commit`（换页）和 `internal_error` 里还会 `cancel_all`，让没开始的旧任务不再白占 CPU。页面已经没了（`self.page` 为空）时回来的结果直接丢。
- 测试（`tests/paint.rs`）：`a_decode_that_finishes_after_navigation_is_dropped`、`the_cache_budget_counts_decodes_in_flight`、`queued_decodes_of_images_that_went_far_are_called_off`，用一个卡在"门"上的假解码器，由测试决定哪个任务什么时候结束。

**3. 同步探头换成读头部**（`qinglan-win/src/images.rs`，`header_size`）：PNG（IHDR）、JPEG（走到 SOFn 段）、GIF（逻辑屏幕）、WebP（`VP8 ` / `VP8L` / `VP8X` 三种）、BMP（`BITMAPCOREHEADER` 和之后的）直接从头几个字节读尺寸，读不出（太短、别的格式、JPEG 在帧头之前就到了扫描段）才交给 WIC。纯安全 Rust，每一步都 `get`，不会越界，JPEG 的循环每圈至少前进 2 字节。`probe` 之前的 SVG、截断检查没动。**行为差别**：一个 WebP 在没有 WebP 解码器的系统上，以前探头就失败（显示 alt 文字），现在探头成功、解码失败，这张图画成空白（盒子大小照常）；探头成功但数据在头部之后坏掉的图，以前是同样的结果。本机有 WebP 解码器。GIF 取的是逻辑屏幕，不是第一帧自己的矩形（WIC 取后者），差别只在极少数裁过的 GIF 上，盒子大小以 `width`/`height` 属性或逻辑屏幕为准，画的时候本来就缩放到盒子里。

**4. 量到的**（`qinglan.exe --bench <中华人民共和国> --replay tests\corpus\local\zhwiki-rec --replay-delay 80 --width 1920 --height 1080`，5 次新进程，**插电 `BatteryStatus` 2**，前两次有别的会话的 1 个 rustc 在跑，后三次没有）：
- `scroll-max-ours-ms`：15.3、12.1、13.0、24.1、22.4（目标 ≤ 50）；`scroll-max-ms`：17.0、13.5、15.0、27.0、25.1；`scroll-present-stalls` 5 次都是 0。
- 滚到底的私有字节：45 805 568、45 772 800、44 732 416、45 699 072、45 432 832，最大 **45.81 MB**（目标 ≤ 46）；峰值 45.58–46.29 MB。比上一步（最大 45.44 MB）多 0.4 MB，仍在 46 MB 之内；两个线程同时解码的临时缓冲没有把它顶过线。
- `scroll-decode-ms`（界面线程上启动解码花的总时间）10.8–12.8；首屏 115.6–228.8 毫秒（首屏上的图不再等解码，但这几次之间噪声大：插电第 1、2 次别的会话在编译）。
- 同一个 bench 在电池、别的会话在编译（1–12 个 rustc）的时候量的 142–462 毫秒（解码已在后台、探头还在界面线程）不能当数字，只说明那时的瓶颈是探头。
- **`qinglan.exe` = 1 969 664 字节，没达到这一步的目标 ≤ 1 965 000（超 4 664 字节）**，在 `PLAN.md` 的上限 1 992 294 之内（余 22 630）。这一步加了池、记账、读头部、窗口一侧的补画，约 15 KB。试过的省法：在途任务表和距离表改成 `Vec`（`FxHashMap` 每多一种键值就多一份实例）、去掉 `mpsc` / `Condvar` / `VecDeque`、排序改成插入、`ByteLru` 的淘汰只留一份、线程只从一个函数起，一共从第一版的 2 012 160 降到 1 969 664；再省要么砍功能（取消、距离优先），要么另找地方（例如把池搬到 "z" 的 `qinglan-win`，估计省 1–2 KB，不够）。**这一项留给确认：放宽到 1 975 000，还是再找一轮体积。**

### 2-0 收尾的决定（2026-10-16，复核后）

- exe 1,969,664 字节，比 2-0 自定的 1,965,000 多 4.7 KB，**接受**：从 2-0 起按第 2 层的预算管（第 2 层做完 ≤ 2.4 MiB，每步报增量）。
- 复核（插电，但另一个会话有 3–6 个编译进程在跑）：中文维基 `--bench` 3 次，界面线程最长事件 19–24 ms、没有贴图卡顿、滚动中 0 次重排、滚到底后 45.6–45.7 MB，达标。
- 首屏新旧交替 8 轮（`qinglan-cli bench`）：最小值 134.4 → 136.8 ms（+1.8%），中位数 145.9 → 160.9 ms；分阶段最小值：解析 30.0 → 32.1、样式 47.1 → 52.7（约 +12%）、首屏排版 47.7 → 44.4。机器忙，结论不可靠；样式变慢可能来自属性值共享池。2a 开工前先在空闲机器上量一次，超过 5% 就修。
- 2-0 的副作用：没有 WebP 解码器的系统上，WebP 图片现在画成空白（以前显示 `alt` 文字）。应改回：解码失败时按坏图处理、显示 `alt`。放进 2a 一起修。


## 第 2 层 · 2a-1 样式速度复测、WebP、WPT 框架、浮动（2026-10-16）

生成内容（`::before`/`::after`、`counter`、`quotes`、`::marker`）**这一小步没有做**，见本节末"没做的"。

### 1. 样式速度复测（2-0 的 DOM 属性值池）

- 量法：`cargo test --release -p qinglan-engine --test perf`，第 1 层末（3b7e8c2，scratchpad 工作树、自己的 `--target-dir`）和 HEAD 的测试程序交替跑 8 轮，每轮取各阶段的中位数和最小值。插电（`BatteryStatus` 2），开始时没有 cargo/rustc/qpdf；跑到最后另一个会话开始编译（进程数 0 → 3），前几轮是干净的。
- **修之前**（前 3 个干净轮）：解析最小值 19.7 / 19.7 / 19.6 ms → 22.1 / 21.9 / 22.9 ms，**约慢 11%**；样式最小值 34.9 → 34.5 ms（持平）。样式没有变慢；变慢的是解析：每个标签多一个 `Vec`（`store_attrs` 另建）和每个属性值一个先分配、再立刻释放的 `String`（以前这些 `String` 一直活到文档丢弃，不在计时里）。
- **修法**：令牌（`TagToken`）的属性是 `Attrs`：一个 `Vec<Attribute>` 加**一个**存所有属性值的 `String`；`Attribute` 里的值是这个串里的 `off/len`。树构建器把这个串整块接到 `Document::attr_pool` 后面，把偏移平移（`store_attrs`），那个 `Vec` 原样成为元素的 `attrs`（`StoredAttr` 现在就是 `Attribute`）。每个有属性的标签一次分配、一次释放，而不是每个属性一次。`afe_token`（重建活动格式化元素）和 `adjust_attrs` 经 `token_attrs` / `Attrs::push` 复制。html5lib 的 tokenizer 和树构建器测试照旧全过。
- **修之后**（8 轮，旧 / 新）：解析 最小值的最小值 19.7 → 20.0（+1.5%），每轮最小值的中位数 21.2 → 20.5（-3%），每轮中位数的中位数 22.3 → 22.3（0%），每轮中位数的最小值 20.4 → 21.7（+6%）；样式 最小值的最小值 34.5 → 35.6（+3.2%），中位数的中位数 38.8 → 39.3（+1.3%）；总计 56.5 → 58.0（+2.7%）。都在 5% 内（噪声约 ±1 ms）。

### 2. 解码失败的图片是坏图，显示 `alt`

2-0 起 WebP 的尺寸由 `header_size` 直接读出，不再问 WIC；没有 WebP 解码器的系统上，图片有了尺寸、盒子也排出来了，解码才失败，结果是一个空白盒子（以前探头失败就是坏图、显示 `alt`）。现在 `ImageStore` 记下解码失败的节点（`take_decode_failed`：同步的 `prepare` 和池里的 `accept` 都记），窗口（`Page::take_decoded`，返回 `(重画的行, 要不要重排)`）和命令行 `render`（排版循环里先 `prepare` 一遍）把它们标成坏图（`ImageSizes::set_broken`，沿用 2-0 的规则：有 `width`/`height` 的盒子保持原样只重画，只显示 `alt` 的图片重排）。测试：`qinglan-cli` 的 `an_image_that_does_not_decode_is_shown_as_its_alt_text`（头部给出 40×30、位图是垃圾的 WebP；去掉修复时失败），引擎的 `a_decode_that_fails_is_reported_once_so_the_host_can_show_the_alt_text`（池和同步两条路）。

### 3. WPT reftest 框架（`qinglan-cli wpt`，`tests/wpt/`，说明见 `tests/wpt/README.md`）

- 来源：稀疏浅克隆 `web-platform-tests/wpt`（提交 3bcf9ec，BSD-3，`tests/wpt/LICENSE.md`），只拷用到的目录，共约 5.9 MB。
- 把 `tests/wpt/` 当作 `http://wpt.test/` 提供（自己的 `Transport`），这样 `/fonts/ahem.css`、`/css/support/…` 这样的根路径能用。**`.xht` 当 HTML 解析**（引擎没有 XML 解析器），提供时去掉 `<![CDATA[` 和 `]]>`（`<style><![CDATA[ … ]]></style>` 在 XML 里是样式，在 HTML 里是垃圾；不去掉的话参考页的样式全丢，通过率少一截）。
- 800×600，测试平台，逐像素；`<meta name=fuzzy>` 取两个范围的上界（最大差、像素数）。多个 `rel=match` 任一个相同即可，`rel=mismatch` 必须都不同，参考页自己的 `rel=match` 链也要符合（最深 3 层）。跳过：有 `<script>`、`reftest-wait`/`reftest-paged`、没有 `rel=match|mismatch`。单个测试 60 秒没完就整个进程退出。
- 失败分类：`classify` 按测试页和参考页里的特性词猜"要第几步的特性"（表格 2a-2、flex/grid 2b、渐变/阴影/圆角等 2c、网页字体/竖排等 2d、SVG/sticky/表单控件 2e、多栏/动画等"不做"、生成内容"2a-1 没做完"），没命中的是 `bug`；`tests/wpt/notes.txt`（路径、类别、原因，TAB 分隔）是人看过之后写的，优先于自动分类。`RESULTS-<目录>.md` 每个目录一份。`passing.txt` 是现在通过的 286 个，`qinglan-cli/tests/wpt.rs` 跑它。

### 4. 浮动（CSS2 9.5、9.5.1、9.5.2、9.4.1、10.6.7，`layout/floats.rs`，替换第 1 层的"当普通块"）

- **盒子树**：浮动元素（`float` ≠ none 的块、`inline-block`、替换元素，不含脱离文档流的）在父盒里是**行内级占位**（和脱离文档流的盒子一样 `Out::Inline`），所以夹在块之间的浮动被包进一个只有占位的匿名块（CSS2 9.5.1 "好像有一个空的匿名块父盒"）。文字盒和行内盒带着元素的样式（含 `float`），`is_float` 只认 `Block`/`InlineBlock`/`Replaced`。
- **每个块格式化上下文一份浮动表**（`FloatSpace`，坐标是上下文根的边框盒左上角；`Layouter::fs`），上下文根（`establishes_bfc`，现在含浮动自己）进入时换成新表、退出时换回来，并且高度 `auto` 的根长到最低的浮动底边（10.6.7）。浮动自己的排版就是 `inline-block` 的（`layout_atomic`：`auto` 宽度取收缩适应，10.3.5），在段落开头和原子行内一起排好（它的行不能夹进所属块的行里），放置时只定位置。
- **放置**（`FloatSpace::place`，9.5.1 的九条）：从 `max(当前行或块的位置, 上一个浮动的顶)` 起，在容器内边之间取被重叠的浮动挤出来的空档（规则 1、2、3、7），放不下就落到下一个重叠浮动的底边（规则 8），左浮靠最左、右浮靠最右（规则 9）；宽过容器又没有浮动挡路的照放（溢出）。规则 4、5、6：不高于当前行或块顶，也不高于上一个浮动的顶。零高零宽的浮动不挤任何人。`clear` 作用在浮动上：不高于所清除的浮动的底。
- **块的位置要猜**：块排版自下而上，块不知道自己的位置（外边距合并要等孩子排完）。浮动要位置，所以父块先按"看得见的外边距"（`est_top_margin`：自己的上外边距加上第一个孩子等等能并上来的）猜一个，在 `Layouter::pos` 里告诉孩子；孩子排完发现真位置不同、而且它看过或改过浮动表（`touched`）时，用 `Mark`（各个输出数组的长度）撤销再排一次。没有浮动的页面不进这些（`has_floats` 为假）。
- **清除间距**（9.5.2）：孩子有 `clear` 时，真位置取 `max(假想位置, 被清除的浮动的底边)`；被清除的浮动是**排这个孩子之前**已有的，不含它自己里面的（clear-005 这类）。外边距怎么和间距相互作用只做了最简单的一种（见没做的）。
- **块格式化上下文的根要躲开浮动**（`overflow` ≠ visible、`flow-root`、浮动之外的 `inline-block` 等本来在流里的）：在假想位置取浮动留下的空档（左移、变窄，`avoid_floats`）；排完后再按它**实际的高度**检查空档，放不下就落到挡路的浮动底边再排（≤ 64 次）。
- **行**（`inline.rs` 的 `lay_lines`）：每行先把行首的浮动放好（行首只能有边、占位和可折叠空格），再用该行高度下的空档排（`line_band`；行高在排之前不知道，先用支柱的，行里有原子行内就用它们的 `a + d`），行起点左移 `line_left`、宽 `line_w`（对齐、两端对齐、悬挂空格都用它）。行中间的浮动：放得进"它前面的内容之后还剩的宽度"就放在这一行顶部再断一次行，放不下就等这行结束后放在下一行顶。第一个不可断片放不进被挤窄的行时，行整个下移到浮动结束处。
- **绘制**（CSS2 附录 E.2 第 5 步的简化）：浮动先攒着，在所属的层叠上下文（根、定位盒、浮动自己、原子行内、替换盒）画完后、按文档顺序画；所以它在其他流内块的背景之上。简化：流内块的文字也在它之下（真正的顺序文字在浮动之上，只有浮动和文字重叠时才不同）。
- **最小/最大内容宽度**：浮动在 `measure_inline` 里，最小宽度各自取最大，最大宽度加进所在的行（`UnitKind::Float`）。
- 命中检测不用改：浮动里的行是它自己的块的行，位置沿 `layout_parent` 链算（浮动的 `layout_parent` 是占位所在的块）。
- `display: flow-root`：第 1 层就是上下文根，现在它里面的浮动被它包住。
- **没做的**：(a) 浮动和外边距合并、清除间距的相互作用（CSS2 8.3.1 / 9.5.2、浏览器对"相邻浮动"的做法）：空块里的浮动按外边距合并之后的位置放，清除间距不把外边距隔开，WPT `adjoining-float-*`、`clear-on-*-with-margins`、`negative-clearance-*`、`margin-collapse-clear-*` 等失败，见 `tests/wpt/notes.txt`；(b) 块格式化上下文的根和浮动之间外边距、高度共同决定能不能放下的几种情形（`new-fc-*`、`floats-wrap-bfc-with-margin-*`）；(c) `white-space: nowrap` 的行和浮动、行内元素里的浮动要算行内框的边（`float-nowrap-*`、`float-in-inline-*`）；(d) 文字在浮动之上的真正绘制顺序；(e) 行高由比支柱大的文字（`<big>`）撑高时，浮动空档按支柱的高度找。(a)、(b)、(c) 的失败每条都在 `notes.txt` 里有原因；其中没有逐个看过的写明"没有逐个分析"。

### 5. WPT 通过率（`qinglan-cli wpt <目录>`）

| 目录 | 通过 / 可判定 | bug 类失败 | 说明 |
|---|---|---|---|
| css/CSS2/floats | 66 / 100（66%） | 22 | 其余在 notes.txt |
| css/CSS2/floats-clear | 133 / 211（63%） | 27 | 同上；多栏的 4 个标"不做" |
| css/CSS2/generated-content | 19 / 205（9%） | 0 | 其余都是"生成内容没做" |
| css/css-lists | 68 / 143（48%） | 2 | 行内列表项的标记，和浮动无关，没有分析 |

bug 类共 51 条，**没有修完**：每条在 `notes.txt` 里有原因（家族原因；没逐个看的写明了）。这一小步在工具调用数限制处停在这里。

### 6. 数字（见本节上面的各项）

`qinglan.exe` = 1 989 120 字节（2-0 末 1 969 664，**本步增量 +19 456 B**；2a 全部的预算 +80 KB，剩 ≈ 62 KB）。zhwiki-rec 1920×1080 滚到底私有字节 5 次 45.49 / 45.58 / 46.19 / 46.55 / 45.64 MB（最大 46.55，预算 50）。首屏（`--bench`，5 次中位数）：zhwiki-rec 147.5 ms，gh-rust 168.8 ms，gh-ripgrep 225.3 ms（**超 200**；这一组跑到一半另一个会话开始编译，不干净）。随后把 2-0 末的程序（92fd668，scratchpad 工作树）和现在的交替跑 6 轮（每轮三个页面，两个程序顺序轮换），但**另一个会话的 5 个 cargo/rustc 进程一直在跑**，每组 6 个数从 130 到 360 ms 散开，只有最小值有参考价值：首屏最小值 旧 → 新：zhwiki-rec 130.2 → 142.7（+9.6%），gh-ripgrep 158.1 → 171.8（+8.7%），gh-rust 159.9 → 203.5（噪声大）；中位数 zhwiki-rec 225 → 285，gh-ripgrep 290 → 290，gh-rust 301 → 324，都被占用的机器淹没。结论：**首屏可能慢了约 9%（浮动的检查：每个块多一个 `Mark`/`est_top_margin` 判断只在有浮动的页面才进；有浮动的页面，如 zhwiki 的缩略图，会多排一次），没有在空闲机器上证实或排除；在 2a-2 开工前先在空闲机器上量一次。**

## 第 2 层 · 2a-1 续：浮动的绘制顺序、生成内容、浮动 WPT（2026-10-16）

接着上一节（`95af212`）。上一节"没做的"里的（d）文字在浮动之上、生成内容，和 WPT 里的浮动 bug 类，这一节做了。

### 1. 浮动的绘制顺序（CSS2 附录 E.2 第 3、4、7 步，`paint/build.rs`）

- 一个层叠上下文（根、定位盒、浮动、原子行内、替换盒）画两遍：第一遍只走流内块的背景和边框（第 3 步），遇到的浮动先攒着；然后每个浮动各自作为一个上下文画（第 4 步）；第二遍只画行（文字、行内盒背景、列表标记、原子行内，第 7 步）。没有缓冲区，没有重排显示项，多走一遍块树的代价可忽略。
- 副作用是对的：负外边距让后一个块的背景盖到前一个块的文字上，现在文字在上（以前是后一个块的背景盖住文字）。`tests/paint.rs` 的 `floats_paint_over_block_backgrounds_and_under_inline_text`（浮动的上半在文字下面、下半盖住后一个块的背景）。
- 完整的层叠上下文（`z-index`、命中检测按绘制顺序）仍是 2a-2。

### 2. 生成内容（CSS2 第 12 章，css-lists-3；`css/`、`layout/generated.rs`、`layout/boxtree.rs`）

- **属性**：`content`、`quotes`（继承）、`counter-reset`、`counter-increment`、`counter-set`，进了 `PropId`（`all` 也复位它们）。`content` 解析 `normal`/`none`、字符串、`attr(名)`、`counter(名[, 样式])`、`counters(名, 分隔[, 样式])`、四个引号关键字、`url()`；`/` 之后的替代文字丢掉。上限：每个声明 64 项，字符串 4096 字符，`attr()` 值 2048 字符，生成文字 8192 字符，同时存在的计数器 4096 个，`counters()` 一次连 64 层，引号嵌套深度 65536。`counter(c, none)` 无效（`none` 不是 `<counter-style-name>`，整个 `content` 作废）。
- **层叠**：`::before`、`::after`、`::marker` 的规则单独存（`RuleSet::pseudo_rules`，同样按 id/class/标签分桶），每个元素算完样式后只在有这种规则时才匹配（没有这类规则的页面一点不多花）；匹配用 `Matcher::matches_owner`（选择器里 `::` 前面的部分对元素匹配）。结果在 `StyleTree::pseudo`，父样式是元素自己的样式（所以继承、`display`、`float`、`position` 都和普通元素一样）。UA 样式表加了 `q::before/::after`（`open-quote`/`close-quote`）。
- **盒子树**：`::before`/`::after` 是元素的第一个/最后一个孩子：`content` 是项目列表才生成盒（`normal`/`none` 不生成；`display: none`/`contents` 也不生成）；盒子匿名（节点 `NONE`），样式是伪元素自己的，文字是普通的 `Text` 盒（`synth`），所以排版、断行（中日韩也是）、`white-space`、`text-transform` 都和文档里的文字一样。连续的文字项合成一个盒。空串 `content: ""` 也生成盒（清除浮动的 `::after { content:""; display:block; clear:both }` 要用）。
- **计数器**（`generated.rs`，CSS2 12.4.1）：一个栈，每个实例记下作用域（父元素的孩子列表，`frame`）和创造它的元素。进入孩子列表时记栈高，离开时截断；`counter-reset` 先删掉同一列表里同名的实例（前面兄弟做的被替换），祖先的留在下面（`counters()` 显示整条链）；`counter-increment`/`counter-set` 找不到计数器就先在这个元素上 `reset 0`；`counter()`/`counters()` 用到不存在的名字也一样。顺序是 reset → increment → set。伪元素的计数器属性像它元素的孩子一样生效。
- **`list-item` 计数器**取代原来的 `next_ordinal`：`ol`/`ul`/`menu` 隐含 `counter-reset: list-item`（`ol start`、`reversed` 按 HTML 4.4.5 折算），列表项隐含 `counter-increment: list-item`（父是 `ol reversed` 则 -1；作者写了 `list-item` 的 increment/set 就不隐含），`li value` 是 `counter-set`。结果和以前相同；改变的是：`display: list-item` 的 `div` 在 `ol` 里继续同一个计数器（以前每个父元素各自从 1 开始），生成的 `display: list-item` 伪元素也有标记。`counter(list-item)` 可用。
- **`::marker { content }`**：`none` 是没有标记，字符串/计数器等替代整个标记（没有后缀 ". "）；`normal` 是原来的标记。只做 `content`：标记框的颜色、字体等不做（`marker-*`、`nested-marker-styling` 这类 WPT 失败，在 notes.txt）。存在 `Rare::marker`，布局的 `lists::marker_string` 统一取。
- **引号**（CSS2 12.3.1）：`open-quote` 取嵌套深度对应的一对，之后深度加一，`close-quote` 先减一再取；深度为 0 的 `close-quote` 什么也不做（css-content-3 3.2）；`no-open-quote`/`no-close-quote` 只改深度；对数超过的深度用最后一对；`quotes: auto` 按 `lang`：日文、zh-TW/HK/Hant 用「」『』，fr « » ‹ ›，de „ " ‚ '，其他 " " ' '。
- **`url()` 图片**：生成一个替换盒（`Replaced`，样式是 `display: inline` 的匿名样式，所以盒子的内边距、边框不套在图上），节点号取文档节点数之后的顺序号（每次排版都一样，宿主按节点号收图）。**不准确的地方**：网址按文档地址解析，不是按样式表的地址（外部样式表里的相对网址会错；2c 的背景图一起解决）。
- **计数器作用域的选择**：CSS2 12.4.1（和 CSS2 的 `content-counter-006/007/008`）说 `counter-reset` 的计数器后面的兄弟都看得到；css-lists-3 4.4.1 的算法（浏览器现在的做法，`css-lists/counter-001`、`counters-scope-002` 是同样的页面、新的期望）是：后面的兄弟从前一个兄弟继承计数器时，如果父元素的计数器里已经有同名的就不继承。**照 css-lists-3 做**，CSS2 那 3 个页面因此不过（notes.txt 有一条，类别 `limit`）。
- 没做：`counter-reset: reversed(名)`（css-lists-3，32 个 WPT 页面）、`counter-style` 的自定义（`@counter-style`，格鲁吉亚、亚美尼亚等命名计数制：退回十进制）、`::marker` 的样式、`::first-line`/`::first-letter`、`content` 里的 `image-set()`/渐变、`display: list-item` 的行内项目的标记。

### 3. 浮动 WPT 里改了的

- **清除间距让空块不再穿透**（`block.rs`）：带 `clear` 的空块，如果真的被清除（位置被浮动底边推下去了），它的上下外边距不再折叠穿过自己：它的位置是清除之后的位置，之后的游标从这里起算，下外边距只剩它自己的（`BlockResult::bot_only`）。这是清除浮动 `::after { content:""; display:block; clear:both }` 的写法，之前父元素高度不含浮动。floats-clear +6。
- **BFC 盒子旁边放得下**（`floats.rs::avoid_floats`）：给了宽度时，宽度超定的外边距里放弃右外边距（10.3.3），只算宽度加左外边距。`new-fc-beside-float-with-margin`。
- **浮动不换序**（`inline.rs`）：同一行里一个浮动因为放不下被推到下一行时，它后面的浮动也一起推（9.5.1 规则 5）。`floats-placement-vertical-003`。
- **比容器宽的浮动**（`floats.rs::place`）：可以从远的一边伸出去，但不能越过另一侧的浮动，旁边有同一侧的浮动（不管在容器里外）就不行（规则 3、7）。`floats-rule3-outside-*`、`floats-rule7-outside-*`。
- **试过没留的**：让"装着浮动的块"不再折叠穿过自己——修好 `new-fc-separates-from-float`，但弄坏 `clear-clearance-calculation-001` 到 `005`、`clear-float-003`、`adjoining-float-nested-forced-clearance-003`，净亏 3 个，撤掉。真正的做法是 LayoutNG 那样带"排除空间"的外边距累积，不在这一步。

### 4. WPT 通过率（`qinglan-cli wpt <目录>`，前 → 后）

| 目录 | 通过 / 可判定 | bug 类 | 说明 |
|---|---|---|---|
| css/CSS2/floats | 66 → 70 / 100 | 22 → 18 | 剩下的每条在 notes.txt（外边距折叠 + 清除间距 + BFC 盒子旁边的家族、nowrap、浮动在行内元素里、静态位置、零宽浮动） |
| css/CSS2/floats-clear | 133 → 139 / 211 | 27 → 17 | 家族同上；5 个是 XHTML 自闭合标签（`limit`），1 个要 2c 背景图 |
| css/CSS2/generated-content | 19 → 181 / 205 | 0 → 2 | 剩下：表格 2a-2（7 个）、bidi 2d（2 个）、XHTML（4 个）、测试平台（3 个）、计数制（2 个）、CSS2 计数器作用域（3 个）、bug 2 个（行尾换行之后的行、计数器 `square` 对列表标记） |
| css/css-lists | 68 → 80 / 143 | 2 → 14 | `reversed()` 32 个（`limit`）、`::marker` 样式 8 个（`limit`）、行内列表项的标记 4 个、计数器继承细节 10 个（`bug (open)`） |

- 分类器不再把"用了生成内容"当成"还没做"（`classify` 里去掉了），所以以前"通过"的页面里有不少是两边都空白的假通过（`css-lists` 的 `counter-reset-reversed-*`、`generated-content` 的几个），现在如实失败；`passing.txt` 重写（470 条，`--check` 全过）。
- 测试平台里每个字 1em 见方，所以 `white-space: pre` 的长行、制表符和等宽字体宽度和真实字体不同的页面（3 个）不是引擎的错，记作 `limit (test platform)`。`.xht` 当 HTML 解析，`<div/>`、`<span />` 这类自闭合写法变成开标签（`limit (xhtml as html)`）。
- `about:blank` 作参考页：harness 补了（以前 404）。

### 5. 数字

- `qinglan.exe` = 2 024 960 字节（上一节末 1 989 120，**本步增量 +35 840 B**；2a 全部 +55 296 B，预算 +80 KB，剩 ≈ 26 KB）。
- zhwiki-rec 1920×1080（`qinglan.exe --bench <网址> --replay tests/corpus/local/zhwiki-rec --width 1920 --height 1080`，插电，新进程 5 次）滚到底私有字节 47 329 280 / 48 209 920 / 47 722 496 / 47 566 848 / 47 464 448（最大 48 209 920 = 45.98 MiB；峰值最大 48 779 264 = 46.52 MiB），预算 50 MB。速度这一步不量（另一个会话在编译，2a 末在安静的机器上统一量）；这 5 次的首屏 188 / 194 / 204 / 148 / ？ms 只是顺带，不作数。
- 栈：`gen_node` 的 `Special` 分支拆成 `gen_special`（`deep_atomic_and_positioned_nesting_fits_in_one_megabyte` 在加了生成内容之后一度栈溢出，调试构建里每层递归的栈帧要小）。

### 6. 没做的 / 留给 2a-2

- 外边距折叠 + 清除间距 + "相邻浮动"的真正做法（带排除空间的外边距累积）：floats-clear 的 17 条和 floats 里的 `new-fc-*`、`floats-wrap-*` 8 条。
- `white-space: nowrap` 的行和浮动、浮动在行内元素里的盒边。
- 表格里的生成内容、层叠上下文和命中检测（2a-2）。
- 行尾保留换行之后的空行（`content-175`）。

## 第 2 层 · 2a-2 表格、层叠顺序、WPT、真实网页（2026-10-17）

接着 2a-1（`26b5823`）。代码在 `layout/table.rs`（新）、`layout/boxtree.rs`（匿名表格对象）、`paint/build.rs`、`interact.rs`、`css/`。

### 1. 盒子树：表格的各个部分和匿名对象（CSS2 17.2.1，`boxtree.rs`）

- 表格部分仍是 `BoxKind::Block`，`fallback` 记着它的 `display`（`role_of` 把它分成 表/行组/行/单元格/列/列组/标题）；`inline-table` 是 `InlineBlock`，`fallback = InlineTable`。
- `finish_block` 先看自己的角色：表、行组、行、列组的孩子过 `fix_table`——不合格的孩子（行组里的单元格、表里的文字……）每一段连续的包进一个匿名行（再包进匿名单元格），表里的行再包进匿名行组；其他盒子里的表格部分（连着的一段）包进匿名表格，**在行内元素里是匿名 `inline-table`**（不把行内盒拆开，`table-anonymous-objects-167` 一类）。表格部分之间的空白文字不产生盒子。
- 表的孩子按这个顺序排：列和列组、标题、第一个 `thead`、其他行组、第一个 `tfoot`（CSS2 17.2：表头总在上、表尾总在下；多出来的 `thead`/`tfoot` 当普通行组）。
- **单元格的内容放进一个匿名块**（`finish_block` 的 `Role::Cell`）：`vertical-align` 是挪这个块的 `y`，不用重排单元格（嵌套表格重排两次会指数增长）。
- `border-collapse`、`border-spacing`、`empty-cells`、`caption-side`（继承，放在 `TextStyle`）和 `table-layout`（`BoxStyle`）进了层叠；UA 样式表：`table { border-spacing: 2px }`、`th { text-align: center }`（浏览器是 `-internal-center`：父元素设了 `text-align` 就听父元素的，没做，`th-text-align` 失败）；`cellspacing` 属性是 `border-spacing` 的提示。

### 2. 表格布局（`layout/table.rs`）

- **格子**（HTML 4.9.12）：单元格占本行第一个空位，`colspan` 上限 1000（HTML 的上限），`rowspan` 上限 65534 且不超出行组（`rowspan=0` 到行组末尾），列数上限 1000，行数上限 65534；只有 `td`/`th` 看这两个属性。格子的数据只有单元格表（位置、跨度），**不是 行×列 的二维数组**，所以大小跟单元格数成正比。排版用的工作量（`charge`）：单元格 + 行 + 列。表格整个排完，不在里面停（第一屏优先的停止线对表格无效，`no_stop`）。
- **列宽（自动布局）**：每个单元格最小/最大内容宽度 = 内容的（`content_widths`）+ 内边距 + 边框；有固定 `width` 的单元格最大宽度 = 这个宽度（最小不变），百分比宽度记在列上（`content-box` 时再加内边距和边框，`fixed-table-layout-025` 一类）。单列单元格定列的最小/最大/固定/百分比；跨列的单元格把需要的部分按各列最大宽度的比例补给它们（先处理跨得少的）。**分配**（css-tables-3 "distributing excess width"、浏览器的做法）：每列从最小宽度起，多出来的先给有百分比的列（到百分比算出来的宽度），再给有固定宽度的列（到固定宽度），再给其余的（到最大宽度），最后还多的按现有宽度比例给自动列（没有自动列给固定列，再给全部）。表宽 `auto` 时取 `min(max(最小, 可用), 最大)`（最大宽度要把百分比列算进去）；有 `width` 取 `width`，但不小于最小宽度；还不小于最宽的标题。
- **固定布局**（17.5.2.1）：`table-layout: fixed` 并且 `width` 不是 `auto`；列宽取 `col` 的宽度、没有就取第一行单元格的宽度（跨列的均分，百分比按表宽），没有宽度的列平分剩下的；单元格盒子宽度就是列宽（内边距比列宽还大时盒子也不比列宽大）。
- **行高**：单元格按列宽排完得到自然高度；行高 = 单元格自然高度、单元格 `height`、`tr` 的 `height` 的最大值，基线对齐的单元格按"基线以上最大 + 基线以下最大"算；跨行的单元格缺的高度平均补给它跨的行；表的 `height`/`min-height` 比内容大时，多的按行高比例补给各行（空表也按它长高）。`vertical-align` 只认 top/middle/bottom/baseline（别的当 baseline）；基线 = 第一行的基线，没有就是内容区底边。
- **标题**（17.4）：表盒子（`BoxOut`）是整个"表包装盒"（标题包括在内），表自己的背景和边框只画格子盒：`LayoutResult::tables[表].y/h`（有标题或合并边框时才有这一项）；标题的宽度 = 表盒宽度（表至少和最宽的标题一样宽）；`caption-side: bottom` 放在下面。标题的相对定位偏移照做。
- **`empty-cells: hide`**（17.6.1.1）：分离边框模型里，内容高度为 0 的单元格不画背景和边框（`BoxOut::hide`）。**没做**：整行都是这样的单元格时这一行没有高度（`empty-cells-applies-to-011/012/017`）。
- **`visibility: collapse`**：只做了行（行和它后面的间距都没有；单元格因为继承 `visibility` 也不画）；列和跨过被折叠的行/列的单元格没做（`visibility-collapse-*`、`column-visibility-*`）。
- 没做：HTML 的 `rules`/`frame` 属性（`rules-groups`）、`col`/`colgroup` 的边框和合并边框冲突里它们的位置、单元格百分比高度（当 `auto`，`percent-height-*`）、表格分页。

### 3. 边框（17.6）

- 分离模型：和块一样画盒子的边框，间距是 `border-spacing`。表格的内边距照常。
- **合并模型**：先把所有格线的边框摊平——表、行组、行、单元格各自的边框放进它们贴着的格线（`Edge`），按 17.6.2.1 选：`hidden` 压倒一切，其余先比宽度，再比样式（`double > solid > dashed > dotted > ridge > outset > groove > inset`），再比谁离单元格近（单元格 > 行 > 行组 > 表），完全一样时先放进去的（左上的单元格）留下。**列和列组的边框不参与**。边的宽度取整数像素（和 Blink 一样，`border_px`）；一条边两边的盒子各得一半，**前面的得小的（向下取整），后面的得大的**（1px 的边：前一个格子 0、后一个 1），表的外边也是（表是"前面"的：上、左边得小的）。表在合并模型里没有内边距，边框宽度就是这样得来的；单元格的边框宽度也用这个（`Flow::cell_bor`）。边的数组（上下各一份）最多 262144 格，更大的表退回分离模型（`MAX_EDGE_SLOTS`），画不出合并边框。
- 画法：排版时把每条边变成矩形（`TableInfo::segs`，相对表盒子，同色同宽相邻的合并），绘制时在表的子孙背景之后画（所以在单元格背景之上、文字之下）；垂直的边在两头伸出相邻水平边宽度的一半，转角不留缺口；表、行组、行、单元格、列自己的边框在合并模型里不画。`double`、`dotted` 等样式按实线画（2c 做）。

### 4. 层叠顺序（CSS2 附录 E，`layout/mod.rs`、`paint/build.rs`、`interact.rs`）

- 层叠上下文：根、有 `z-index`（不是 `auto`）的定位盒、`position: fixed`（`makes_context`；不安排 `opacity`/`transform`，2c 往 `makes_context` 加一行就行）。每个定位盒记它属于哪个上下文（`ctx`：沿 `parent` 往上第一个造上下文的盒子）和是不是 `z-index: auto`；`LayoutResult::stacked` 按 (上下文, z-index, 文档顺序) 排好，`stacked_in(ctx)` 二分取一个上下文的那一段。`below`/`above`（全局一份）留着，布局转储用。
- 画法：一个上下文的顺序是：自己的背景和边框、负 `z-index` 的孩子（按 z、文档顺序，每个孩子是自己的上下文）、流内块的背景和边框、浮动、流内行内内容、`z-index: auto`/0 的定位盒（按文档顺序；`auto` 的像一个上下文那样画，但它里面的定位盒属于外面的上下文）、正的。`fixed_below`（固定盒画在页面下面的那一段）现在是根的负 z-index 孩子画完之后的位置。`positioned()` 画完恢复调用前的裁剪和固定标志（嵌套上下文里不丢）。
- **点击检测**（`link_at`）：`PageIndex` 按上面的顺序给每个上下文的"流内内容"和每个 `z-index: auto` 定位盒编一个号（`rank_context`，后画的号大）；一个字块的号 = 沿 `parent` 往上找到的第一个有号的盒子的号；碰到的字块里号最大的赢，号相同取树里靠后的。只有文字和原子行内（图片等）参与点击检测（和以前一样）：`z-index` 更高的盒子里的文字或图片盖在链接上面时，下面的链接点不中；盖在上面的只是背景、没有文字，下面的链接仍然点得中（没做"背景挡住鼠标"）。

### 5. WPT（`qinglan-cli wpt <目录>`，提交 3bcf9ec）

| 目录 | 通过 / 可判定 | bug 类 | 说明 |
|---|---|---|---|
| css/CSS2/tables | 219 / 250（87.6%） | 26 | 每条在 notes.txt（`bug (open)` 写了家族原因；没逐个看的写"没分析"）；其余失败：bug (open) 26，not planned 1，step 2c 4 |
| css/CSS2/zindex | 29 / 29（100%） | 0 | |
| css/css-tables | 89 / 140（63.6%） | 31 | 同上；其余失败是后面步骤的特性（分类见 `RESULTS-*.md`：bug (open) 31，limit (print) 2，step 2b 1，step 2c 7，step 2d 6，step 2e 4），打印分页 2 条记 `limit (print)` |

- 拷了 419 个有 `rel=match|mismatch`、没有 `<script>` 的页面和它们引用的文件（`tests/wpt/` 共 2.7 MB）。分类器去掉了"用了表格 = 2a-2"那条（`classify`），所以表格的失败现在算 bug。
- 修了的（在这一步里，通过数从第一次的 164 / 250、80 / 140 涨到现在）：空白在匿名表格的两个部分之间截断了连续段（一个匿名表被拆成好几个）；`display: table` 的空表有 `height`/`width`；单元格百分比宽度要加内边距和边框；列元素的百分比宽度（固定布局）；单元格盒子不比列宽大；行内元素里的表格部分做匿名 `inline-table`；`inline-table` 的基线是第一行的；表至少和最宽的标题一样宽；`min-height`；标题的相对定位；行的 `visibility: collapse`。
- `passing.txt` 807 条，`--check` 全过（以前的 470 条一条没退步）。

### 6. 没做的 / 留给后面

- `visibility: collapse` 的列、整行 `empty-cells: hide` 的行、`rules`/`frame`、列和列组的边框、单元格百分比高度、`th` 的 `-internal-center`、标题的最小宽度算进 `content_widths`（嵌套里）、表格的 `clip` 用样式边框不是合并后的（`overflow: hidden` 的合并表格差 1px 左右）。
- 表格里 `position: relative` 的单元格/行/行组的偏移没有做（只有标题做了）。

## 第 2 层 · 2a-3 内存和体积（2026-10-17）

起点 `9bc25cd`（代码同 `e956d85`）：zhwiki-rec 滚到底私有字节 47.9–50.3 MB（5 次新进程，最大 50.3），`qinglan.exe` 2 134 016 字节（2a 预算 ≤ 2 051 584）。目标：内存最大值 ≤ 48 MB（十进制），体积尽量回到预算内，速度不差过 5%，不删任何功能。

### 1. 内存：自己的分配器（`qinglan-win/src/sys/heap.rs`，`PoolAlloc`）

- **量出来的问题**（`mem-stats` 版，窗口里的 `--bench --mem`）：活堆 33.4 MB（请求字节），Windows 堆已提交 46.3 MB，另有约 3 MB 非堆；差的 12 MB 是 16 字节块头（16.4 万个活块 ≈ 2.6 MB）加上堆里留着不还的空闲块（解析、样式阶段的临时对象，滚动时释放的解码图片，数组倍增留下的洞）。数据结构本身一项最大的才 3–4 MB，逐项压缩治不了这 12 MB，所以改分配器。
- **三层**：① 小块（1–1024 字节，对齐 ≤ 8）：40 个大小等级（128 以内 8 字节一级，之后 16、32、64 字节一级），每级从 16 KiB 的 slab 里分，slab 头 64 字节（空闲链、未用的尾巴、活块数），块本身没有头——Rust 的 `dealloc` 带着布局，大小等级不用存；块属于哪个 slab 看地址（`addr & !(16 KiB-1)`）。slab 满了离开本级的链，释放一个块又回来，没有活块的 slab 解除提交（每级留最后一个，另有最多 32 个空 slab 放在一个公共的备用栈里，一次提交 4 个 slab 以省系统调用）。② 大块（≥ 128 KiB）：自己的 `VirtualAlloc` 页，释放就还给系统（Windows 堆把这个范围的块留在自己的段里，倍增的 `Vec` 会留下一串洞）。③ 其余和分配不到时（slab 的 512 MiB 地址范围用完、系统不给提交）：`System`。块还回哪里只看布局和地址，`realloc` 跨档时搬家（写这个时犯过一个错：大块缩小到小块档时仍留在 `System` 里，`dealloc` 就把它交给 slab，测试里的多线程随机分配抓到了）。
- **锁**：每级一把自旋锁，slab 地址范围一把（先拿级锁再拿范围锁，反过来不拿）；先用的是 `Mutex`，gh-ripgrep 的样式表是多线程并行解析的，`Mutex` 争用时线程睡眠再唤醒，样式阶段中位数慢 5–6%，换成"自旋 200 次再让出时间片"的自旋锁后反而快 15%。分配器里不分配、不 panic（`pool_report` 一度在持锁时 `format!`，自己等自己，死锁，改成先计数后格式化）。
- **`unsafe`** 只在 `sys/heap.rs`，每处注明为什么安全；`qinglan`、`qinglan-cli` 两个二进制包里 `#[global_allocator]` 指向 `PoolAlloc`（开了 `mem-stats` 时是包着它的 `CountingAlloc`）。测试：大小等级覆盖全部 1–1024 字节；4 个线程随机分配、填充、`realloc` 涨缩（含 16 对齐和 ≥ 128 KiB 的块）、释放，查字节有没有被别人改；30 000 个块分配又释放后 slab 是否解除提交、之后能否复用（`sys::heap::tests`）。引擎的集成测试（`hostile`、`perf`）用的是系统分配器，不经过 `PoolAlloc`：hostile 页（`bad_utf8` 4 MB 坏字节，峰值 ≤ 120 MB 的断言）没有在 `PoolAlloc` 下重测。
- **试过没采用**：在 `qinglan.exe` 里嵌清单 `<heapType>SegmentHeap</heapType>`（Windows 的分段堆，不用改代码）：滚到底 43.2–43.5 MB，但整页布局 283 → 397 ms（+40%），峰值 49.9 → 55–56 MB，不采用；只有小块池（块不还系统）：均值 49.3 → 47.0 MB，gh-ripgrep 滚到底反而 30 → 54 MB（样式阶段的小块永远占着），所以才有 slab 解除提交；第一版"块不还"加大块 `VirtualAlloc`：均值 46.4。
- **结果**（`qinglan.exe --bench <网址> --replay tests\corpus\local\zhwiki-rec --width 1920 --height 1080`，新进程各 5 次，十进制 MB）：

| | 滚到底私有字节 | 峰值 |
|---|---|---|
| 起点 | 47.90、49.51、49.27、49.57、50.32（最大 **50.32**） | 49.5–50.4 |
| 小块池（块不还） | 47.02、46.53、48.62、45.67、47.08 | 48.2–49.0 |
| + 大块 `VirtualAlloc` | 45.87、46.34、46.62、46.43、46.71 | 48.3–48.7 |
| 最终（slab 解除提交 + 备用栈 + 自旋锁） | 44.26、43.99、44.59、44.13、44.37（最大 **44.59**，目标 ≤ 48） | 45.5–46.1 |

  前三项节省：小块去掉块头和块内取整、不再留空闲块洞（约 2.3 MB）；slab 解除提交把阶段性的小块还给系统（约 2.1 MB）；大块自己的页（约 0.6 MB，并且让 5 次的离散从 2.4 MB 降到 0.7 MB）。其他页：gh-ripgrep 滚到底 30.4–33.2 → 25.2–27.3 MB，峰值 67.8–68.3 → 62.0–63.4；gh-rust 28.7–29.7 → 25.9–26.4。量时另一个会话的编译器偶尔在跑（内存不受 CPU 影响，但图片到达次序会）。

### 2. 体积：链接映射、排序去重、内联阈值

- **怎么量**：`cargo rustc --release -p qinglan --target-dir <另一个目录> -- -C link-arg=/MAP:<文件>`，要 v0 符号名就用 `RUSTFLAGS="-C target-feature=+crt-static -C symbol-mangling-version=v0"`。**注意**：环境变量 `RUSTFLAGS` 会盖掉 `.cargo/config.toml` 里的 `rustflags`，只写 v0 会丢掉 `+crt-static`，exe 立刻小 120 KB——因为改成动态链接 `VCRUNTIME140.dll`，那不是省，是多了一个依赖。
- **映射结果**（2a 末）：`.text` 1 533 KB，引擎 913 KB（`css::parse_prop` 90、`cascade` 82、`layout::inline` 62、`html::treebuilder` 61、`layout::table` 56、`layout::boxtree` 51）、`core` 240（`drop_glue` 87、**`slice::sort` 66**）、窗口 app 124、hashbrown 57；`.rdata` 159，`.xdata`+`.pdata` 204（unwind 表，崩溃保护的代价）。Debug/fmt 总共只有 8 KB。
- **排序去重**（`qinglan-engine/src/sorting.rs`）：`sort_by_key`/`sort_by` 对每个闭包类型各生成一整份排序代码（6–15 KB），十处共约 66 KB。新函数 `sort_by_u64_key` 把键放进 `(键, 下标)` 对，交给唯一一份非泛型的 `sort_unstable`（下标唯一所以结果等于稳定排序），再按置换的环把元素挪到位；`f32_key`/`f64_key` 给出和 `total_cmp` 同序的整数键。改了 7 处：`parse_cvs_many`、`finish_table_parts`、`select_source`、`paint_order` 的两处、`size_columns`、窗口 bench 的两处。样式层叠里每元素一次的排序（`cascade.rs`）没动，保持原样。省 48 128 字节。
- **内联阈值**：`.cargo/config.toml` 的 `rustflags` 加 `-C llvm-args=-inline-threshold=100`（LLVM 默认 225）。整体 LTO 时最后一遍内联少做一些：省 165 888 字节（7.8%），速度不退（见下），因为分配器已经让解析、样式、排版快了 10–20%。阈值 150 的体积 2 044 928，速度和 100 没有可见差别，所以用 100。没动 `opt-level` 和 LTO 设置。
- **结果**：`qinglan.exe` 2 134 016 → **1 927 168**（−206 848；2a 预算 2 051 584，余 124 416）。分配器本身 +512 字节。这一步多出来的余量留给后面的步骤。

### 3. 速度（`qinglan-cli bench`，1920×1080，旧 `e956d85`（暂存工作树，自己的 `--target-dir`）和新交替跑，只算插电（`BatteryStatus = 2`）、前后都没有 cargo/rustc/link/cl/qpdf 的轮次）

最后一版（自旋锁）3 个干净轮次（另一个会话经常在编译，15 次尝试里只有 3 次干净），最小值 / 中位数，旧 → 新：zhwiki-rec 解析 −21% / −17%、样式 −11% / −15%、首屏 −8% / −1.6%、整页布局 −10.7% / −6.9%、滚动中位数 −1.9% / 0.0%、p95 0.0% / 0.0%；gh-ripgrep 解析 −15% / −19%、样式 −15% / −14%、首屏 −15% / −15%、整页布局 −16% / −16%、滚动中位数 −2.8% / 0.0%、p95 −13% / −4.4%。前一版（`Mutex`）5 个干净轮次：gh-ripgrep 中位数样式 +6.1%、首屏 +5.2%、整页 +5.9%（最小值 −0.3%…−2.8%），所以换了自旋锁。滚动（光栅）这一步没动，三版都在 ±4% 内。

### 4. 没做的 / 留给后面

- 数据结构的压缩没做（盒子 64 字节、盒位置 32 字节、DOM 节点 72 字节、显示列表 3.6 MB）：分配器已经把余量做到 3.4 MB，再压要动很多用法。
- `PoolAlloc` 下没重跑 hostile 页（见上）；`qinglan-cli` 的 `wpt.rs` 里 4 处排序没动（不在 `qinglan.exe` 里）。

## 第 2 层 · 2b-1 flex（2026-10-17）

接着 2a-3（`9e8fbf8`）。代码：`layout/flex.rs`（新）、`css/`（新属性组）、`layout/boxtree.rs`、`layout/block.rs`、`layout/position.rs`、`layout/intrinsic.rs`；测试：`tests/pages/flex-*.html`、`tests/flex.rs`、`tests/wpt/css/css-flexbox`。规范原文在 `docs/spec/css-flexbox-1.html`（只放本地）。

### 1. 样式（`css/types.rs`、`computed.rs`、`parse_prop.rs`）

- 新属性组 `Group::Flex`，`FlexStyle`（`Rc`，不继承，没写时共用初始值，每个元素只多一个指针）：`flex-direction`、`flex-wrap`、`order`、`flex-grow`、`flex-shrink`、`flex-basis`、`justify-content`、`align-items`、`align-self`、`align-content`、`row-gap`、`column-gap`。简写 `flex`（`none`/`auto`、1–3 个值；省略的 `flex-basis` 是 `0%`，规范原文，不是 `0px`）、`flex-flow`、`gap`/`grid-gap`；别名 `grid-row-gap`、`grid-column-gap` 和 `-webkit-flex*`、`-webkit-order`、`-webkit-justify-content`、`-webkit-align-*`（老中文站点常见）。对齐值带 `safe`/`unsafe`，`first baseline`/`last baseline` 两个词。
- **blockify**（4）：父元素是 `flex`/`inline-flex` 时，`compute_box` 把子元素的 `display` blockify、`float` 取消（没声明任何盒属性的元素也走这条，不能再共用初始盒样式）。`display: contents` 底下的孩子不处理（它们的父是 `contents`）。
- **flex 项的 `z-index`**（4.3）：静态定位的 flex 项有 `z-index` 时按 `position: relative` 算（没有偏移，画法和层叠一样）。近似：它会变成绝对定位后代的包含块。

### 2. 盒子树（`boxtree.rs`）

- `finish_flex`：flex 容器的孩子是 flex 项；每一段文字（和 `display: contents` 带来的行内盒）包进一个匿名块；**只有空白的一段不渲染**，不管 `white-space`（4）；脱离文档流的孩子留作占位。孩子按 `order` 稳定排序，所以**绘制顺序和点击顺序**自然是排序后的（5.4、附录 E）。`LayoutBox::flex_item` 让项成为块格式化上下文的根（外边距不和孩子合并）。

### 3. 排版（`layout/flex.rs`，规范 9 的顺序）

- 容器和表格一样整个排完（`no_stop`），按块格式化上下文根处理（躲浮动、`clear` 之后重新取空档：`block.rs` 里清除间距移动后重算 `avoid_floats`）。盒子自己的宽高、外边距、内边距用 `block_geometry`（`Geom` 的字段改成 `pub(super)`）。
- **逻辑坐标**：`Axes` 把主轴/交叉轴的起点终点对应到物理边（`row-reverse`、`column-reverse`、`wrap-reverse`、`direction: rtl` 的行、rtl 的列交叉轴），放位置最后一步才换成物理坐标。
- **项的大小**（9.2 第 3 步、4.5）：行方向 flex 基准尺寸来自 `width`/`flex-basis`，没有就是 `content_widths` 的最大内容宽度（带备忘，不排版）；自动最小尺寸 = 最小内容宽度，有定宽时取两者小的，再受 `max-width` 限制，滚动容器（`overflow` 非 visible/clip）是 0。列方向要内容高度：`flex_measure` 把项排一遍再撤销（`Mark`/`restore`，`Flow::ignore_h` 让它不看自己的 `height`/`min-height`/`max-height`），按（盒、宽度）记一项（类型和 `widths` 一样，不多一份哈希表的代码），所以嵌套容器每层只量一次。替换元素有比例时，列里的高度按宽度折算。
- **9.7 解弹性长度**：按规范逐步写（冻结不弹性的项、剩余空间、`sum<1` 时乘、最小/最大违规按总违规冻结），整数（1/64 px）运算。
- **9.3 分行、9.4 交叉尺寸、9.6**：行的交叉尺寸 = 最高的项，或基线对齐时"最高的上半部 + 最深的下半部"；单行且容器交叉尺寸确定时 = 容器内部尺寸（行内项直接按这个高度排，不用重排）；`align-content` 多行（包括 `wrap-reverse`）；`stretch` 的项（`auto` 交叉尺寸、没有 `auto` 外边距）长到行高并受 min/max 限制；`auto` 外边距吃掉主轴/交叉轴剩余空间；`safe`/`unsafe`；`gap`（百分比按内容框，高度不确定时按 0）。
- **拉伸怎样不指数增长**：行容器的项在最终宽度排一遍；某项被拉高时，只有"高度变了内容也变"的项（本身是 flex 容器或表格、有百分比 `height`/`min-height`/`max-height` 的后代、有脱离文档流的后代，`height_sensitive`，按盒备忘）才整行重排，其他的直接把盒子拉高；重排用掉的工作量上限是整页预算的 1/8（`FLEX_REPEAT_SHARE`），超了就只拉高盒子。列容器的项最后排一遍（高度已知）。实测见下。
- 9.8 确定尺寸：列项的"排后主轴尺寸"只在容器主轴尺寸确定或 `flex-basis` 确定时当作确定（给 `forced_h`），否则用 `ignore_h` 按内容排，百分比高度的孩子不按它解析（`percentage-heights-016`）。
- **9.9 内在尺寸**（`flex_widths`）：行容器最大内容 = 各项贡献之和 + 间距，最小内容 = 贡献之和（单行）或最大的贡献（多行）；列容器取最大的贡献。贡献 = `outer_widths`（宽度或内容），**不能长大**（`flex-grow: 0`）且 `flex-basis` 是定长的项用 flex 基准尺寸夹在 min/max 之间（`intrinsic-size/row-006/007`）；`min-width: min-content|max-content` 关键字抬高贡献。没做 9.9.1.1 的理想算法（规范说它网页不兼容），也没做多行行容器交叉尺寸的"约束下重排"（用单行的式子）。
- **4.1 脱离文档流的孩子**：`Pending` 多了 `align`（`StaticAlign`）：静态位置矩形是容器内容框，按 `justify-content` 和（自己的或容器的）`align-self` 当唯一项摆；`left/right/top/bottom` 都 `auto` 的方向才用。
- **8.5 容器基线**：行里有 `align-self: baseline` 的项就取它们的共同基线，否则取第一项（最后基线同理），记在 `flex_base`；`first_baseline`/`last_baseline` 先看它，`inline-flex` 在行里按第一基线对齐。
- **4.4 `visibility: collapse`（简化）**：折叠的项主轴尺寸算 0、外边距算 0，交叉尺寸照常（就是它的"支柱"），排版仍按原宽度，画法不画。没做规范的"整个算法重来一遍"，所以多行容器里折叠后重新分行不对。
- **对齐细节**：`space-around`/`space-evenly` 没有空间时按 `safe center`（= `flex-start`，WPT 的两个测试一致，和 flexbox-1 的 `center` 不同）；`space-*` 的除法四舍五入（和 Edge 逐像素一样）；列容器里 `left`/`right` 当 `start`（css-align-3 6.3）。
- 替换元素、表格当 flex 项：表格（`table.rs`）认 `Flow::forced_h` 和 `ignore_h`。

### 4. 测试工具（`qinglan-cli wpt`）

- 分类器：`display:flex` 之类不再算"后面的步骤"（只有 grid 还是 `step 2b`，2b-2 做）；`text-wrap`、`aspect-ratio`、`flex-wrap:balance` 归"没计划"。
- XHTML 测试（`.xht`/`.xhtml`，当 HTML 解析）：除了去掉 `CDATA`，现在还去掉 XML 注释（`<style>` 里夹的 `<!-- -->` 会吞掉后面的规则），并且把自闭合的非空元素补成 `<div></div>`。老目录因此多过了几条：floats-clear 167 → 172、generated-content 186 → 188、css-tables 89 → 90，没有一条退步（`passing.txt` 846 → 1501 条）。
- `tests/wpt/css/css-flexbox`：只拷有 `rel=match|mismatch` 又没有 `<script>` 的页面和它们引用的文件（1491 个文件，1.8 MB；`tests/wpt` 共 4.4 MB），提交 `3bcf9ec`（和以前一样）。

### 5. WPT `css/css-flexbox`（`qinglan-cli wpt css/css-flexbox`）

647 / 951 通过（68.0%，第一版 581）。没通过的 304 条：bug 类 87（`bug (open)`，每条在 `notes.txt` 有一行，按家族写原因，没逐个看的写明"没逐个分析"）、limit (test platform) 26（`flexbox_flex-*`：测试靠默认字体里真实文字的宽度，测试平台每字 1em，单词比盒子宽，自动最小尺寸不让它缩）、limit (placeholder boxes) 12（canvas、iframe、video、object 是灰框，fieldset 没特殊排版）、not planned 51（`flex-wrap: balance`（flexbox-2）、`aspect-ratio`、`text-wrap`）、后面的步骤 128（2b-2 grid 4、2c 30、2d 69、2e 25）。bug 类里家族：带比例的图片当 flex 项（转移尺寸，4.5、9.2 B）、表格当 flex 项、基线对齐里字号混用、9.8 的确定尺寸（拉伸后孩子的百分比高度）、多行/列容器的内在尺寸、`visibility: collapse` 的第二轮。

### 6. 数字（release，插电 `BatteryStatus = 2`，没有别的 cargo/rustc/qpdf 在跑）

- **体积**：`qinglan.exe` 1 927 168 → **1 984 000**（**+56 832**，比 2b-1 的 +50 KB 多 6.8 KB；2b 一共分到 +100 KB，grid 还剩约 43 KB）。链接映射里 `layout_block` 31 KB（`layout_flex`、`flex_inner`、`flex_item` 都内联进去），`flex_widths` 3 KB；`parse_value` 21 KB。已经把两个新哈希表（`measured`、`flex_base`）改成和 `widths` 同一种类型，省 2.9 KB。
- **内存**（zhwiki-rec，1920×1080，`qinglan.exe --bench`，5 次新进程，滚到底的峰值私有内存）：46 415 872 / 46 166 016 / 46 514 176 / 46 063 616 / 46 309 376，**最大 46.51 MB**（≤ 50）。
- **首屏**（`qinglan.exe --bench`，5 次中位数）：zhwiki-rec 113.1 毫秒（121.2 / 110.5 / 105.7 / 116.6 / 113.1），gh-ripgrep 169.8（169.8 / 158.1 / 166.0 / 170.2 / 183.4），gh-rust 148.6（219.7 / 148.6 / 141.1 / 135.1 / 157.0）；都 ≤ 200。flex 容器里首屏停止线不起作用（整个排完），GitHub 页面的首屏因此比 zhwiki 慢。
- **嵌套**（`tests/flex.rs`，工作量 = 排版预算用掉的单位，深 5 → 深 20）：行里嵌行 228 → 818；列里嵌列（每层 `flex: 1`、顶层定高）1 050 → 10 370；每层都有个更高的兄弟逼着嵌套容器重排 1 088 → 12 068。随深度多项式增长（量大的是列方向的 `flex_measure`），不是指数；预算 20 000 000，用掉 0.06%。
- 测试：`tests/pages/flex-row.html`、`flex-column.html`、`flex-misc.html`（手算数字 3 页，规范条款写在页头）；`fallbacks.html` 去掉 flex 部分；`tests/flex.rs`（属性解析、弹性分配、嵌套工作量）。

### 7. 没做 / 留给后面

- `aspect-ratio` 属性、图片等带比例的 flex 项的转移尺寸；表格当 flex 项的尺寸细节；`canvas`/`iframe` 的真内容（2e）；`writing-mode` 竖排的 flex（2d）；规范 9.9.1.1 的理想内在尺寸算法；`visibility: collapse` 的第二轮；flex 项 `z-index` 的"变 relative"是近似。
- 首屏停止线不进 flex 容器：整页都包在 flex 里的页面（现代站点常见）首屏要排完整页。有预算压力时可以让 nowrap 的列容器在已知高度之后停（2b-2 或之后看数字再定）。


## 第 2 层 · 2b-2 grid（2026-10-17）

接着 2b-1（`ad58292`）。代码：`css/parse_grid.rs`（新）、`css/types.rs`、`computed.rs`、`parse_prop.rs`（新属性组 `Group::Grid`、`GridStyle`）、`layout/grid.rs`（新）、`layout/flex.rs`、`layout/mod.rs`、`layout/block.rs`、`layout/boxtree.rs`、`layout/intrinsic.rs`；测试：`tests/pages/grid-*.html`、`tests/grid.rs`、`tests/firstscreen.rs`、`tests/wpt/css/css-grid`。规范原文在 `docs/spec/css-grid-1.html`（只放本地）。

### 1. 样式

- 新属性 12 个：`justify-items`、`justify-self`（进 `FlexStyle`）和 `grid-template-columns/rows/areas`、`grid-auto-columns/rows/flow`、`grid-row/column-start/end`（进 `GridStyle`，不继承，没写时共用初始值）。简写：`grid-template`、`grid`、`grid-area`、`grid-row`、`grid-column`、`place-items`、`place-self`、`place-content`。轨道列表支持 `repeat()`（整数展开，`auto-fill`/`auto-fit` 留到排版时按容器大小定个数）、`minmax()`、`fit-content()`、`min-content`/`max-content`、`fr`、`[线名]`。`subgrid`、`masonry` 不认（声明无效）。
- **上限**（记在 `GRID_MAX_TRACKS`）：每个方向的轨道（显式加隐式）和 `repeat()` 展开的个数都是 1000 条（Blink、Gecko 约 10000；占用图是 行×列 个比特，1000×1000 约 125 KB，所以取 1000）。超出的 `repeat()` 在 1000 处截断，超出的自动放置项不排版。工作量：放置和轨道尺寸按项数×跨度计入排版预算。
- grid 项和 flex 项一样 blockify、取消浮动、按 `order` 排序、`z-index` 当 `position: relative`；盒子树复用 `finish_flex`（文字段包进匿名项，只有空白的不渲染）。

### 2. 排版（`layout/grid.rs`）

- 顺序：显式网格（`auto-fill` 个数按 7.2.3.2，轨道最小 1px 防除零）→ 项的线（8.3、8.3.1，命名线、`name-start`/`name-end` 区域线、负数线、`span`、不够的线当隐式线都有名字）→ 自动放置（8.5，稀疏和 `dense`，`grid-auto-flow: column`，四步按规范）→ 轨道尺寸（11.4–11.8：非跨行的先做，跨行的按跨度从小到大，每步按 11.5.1 先到限制再到限制之外，再是跨弹性轨道的项，最大化，展开 `fr`（`find_fr`，内容尺寸下取各项的最大值，受容器 `min-height`/`max-height` 约束），拉伸 `auto` 轨道）→ 对齐（10：`justify/align-content`，`justify/align-items/self`，自动外边距，`safe`/`unsafe`，`direction: rtl` 把列镜像）。
- **6.6 自动最小尺寸**：跨的轨道里有 `auto` 最小值、跨多条时没有弹性轨道，才用内容最小尺寸；全是定长最大值时取其和为上限。`fr` 的整数余数按小数部分从大到小每条补 1/64 px，所以 99 条 `1fr` 加起来正好是容器宽。
- **不指数增长**：列用内在宽度（有备忘，不排版）；行需要项在最终宽度的高度时用 `flex_measure`（和 flex 同一个备忘，按（盒、宽度）记一项，嵌套每层只量一次），再最终排一次；固定尺寸行里的项不量。实测（`tests/grid.rs`，工作量 = 排版预算用掉的单位，深 5 → 深 20）：列里嵌 2485 → 31689，行（`auto 1fr` + `min-height`）里嵌 1835 → 24400，跨三列的嵌套 2731 → 36751；随深度多项式增长，预算 20 000 000，用掉 0.18%。
- **脱离文档流的孩子**（9）：包含块是网格容器时，包含块是项的线所指的网格区域（`auto` 或不存在的线 = 内边距边），静态位置在该区域里按 `justify-self`/`align-self` 对齐（规范 9.2 写的是内容框，各浏览器和 WPT 的 `positioned-grid-items-*` 都是区域，选区域；两个方向的线都是 `auto` 时才用内容框）；包含块是别的盒子时，静态位置在内容框里。
- **容器基线**：第一行的第一项、最后一行的最后一项的基线。
- **没做**：基线对齐（`baseline` 当 `start`，没有基线共享组和"shim"）、`subgrid`、竖排、`masonry`、网格容器的 `grid-template` 用百分比的循环依赖（百分比轨道在不定高时当 `auto`，不再重算）、行的内在尺寸用的循环百分比。

### 3. 首屏优先进入 flex 和 grid

停止线以前不进 flex 容器（整个排完）；现在在下面的条件下进去（条件不满足就和以前一样整个排完）。**原则**：只有上方的东西不依赖被截掉的部分才能截；被截的盒子高度是下界（高度 ≥ 停止线以上的部分），所以它后面的内容一定在线以下。每个项有自己的停止线（`item_stop`：并排的项不能互相截断，项的 `origin_y` 取估计的顶）。

- **列 flex**（不反向、不换行、高度不定、没有 `max-height`、`min-height` 在停止线以内）：项一个接一个，没有剩余空间，所以起点在停止线以下的项和后面的项不量、不排；每个项量内容高度时也带停止线。
- **行 flex**（一行、不换行、各项 `align` 是拉伸或顶端对齐）：把停止线传进每个项（项的顶是容器内容框的顶）。
- **grid**（高度不定、至多一条弹性行、`align-content` 和各项 `align-self` 是顶端或拉伸、`min-height` 在停止线以内）：量行高时按"行的起点下界"（前面定长行之和）带停止线，起点一定在线以下的项不量；最终排版时用真实的行起点，线以下的项不排。
- 被截的容器记进 `cut`（里面的绝对定位孩子等完整排版）。`tests/firstscreen.rs` 的相等测试（各页 25%/50%/75%、中文维基）通过；新的 `flex_and_grid_shells_are_cut_by_the_stop_line`：列 flex 的工作量降到 1/10 以下，行 flex 和 grid 外壳降到 1/2 以下（行和 grid 的外壳还要读全部文字来算项的最小内容宽度，这部分省不掉）。已知的近似：被拉伸的项里有百分比高度的后代时，行高是下界，后代的高度可能和完整排版不一样（相等测试没有碰到）。

### 4. WPT `css/css-grid`

554 / 1573 通过（35.2%；`qinglan-cli wpt css/css-grid`）。没通过的 1019 条：bug 类 79（每条在 `notes.txt` 有一行：基线对齐 38，其余是表格额外高度的分配 4（不是 grid）、百分比 gap 4、grid 项的绘制顺序 5、替换元素百分比高度的循环依赖 2、百分比行高 4、脱离文档流 9、没逐个分析 13）、limit 23（canvas/marquee/iframe 占位框 19，`::first-line`/`::first-letter` 4）、not planned 836（`subgrid`、`masonry`/`grid-lanes`、`aspect-ratio`、多栏……）、后面的步骤 81（2c 7、2d 64、2e 10）。分类器改了两处：grid 不再算"后面的步骤"；`columns:` 不再匹配 `grid-template-columns:`（之前 555 条被当成多栏）。顺带修的引擎问题：`auto-fit` 的折叠按方向分别记、`fr` 余数、自动最小尺寸、拉伸的项受 `min/max-height` 限制（表格不自己夹）。

### 5. 数字（release，插电 `BatteryStatus = 2`，没有别的 cargo/rustc/qpdf 在跑）

- **体积**：`qinglan.exe` 1 984 000 → **2 138 624**（**+154 624**）。**超出 2b 的预算**（2 027 000，即 2b 总共 +100 KB）111 624 字节：grid 的实现（解析、放置、轨道尺寸算法、对齐）比 2b 给 grid 留的 43 KB 大得多。按任务单的要求没有删功能；2c–2e 还剩的体积要重新分配（第 2 层总上限 2 516 582，现在用了 2 138 624，剩 377 958）。
- **内存**（zhwiki-rec，`qinglan.exe --bench`，5 次新进程，滚到底的私有字节）：47 857 664 / 46 026 752 / 44 736 512 / 44 351 488 / 45 240 320，**最大 47.86 MB**（≤ 50）。
- **首屏**（5 次中位数）：zhwiki-rec 146.6 毫秒（180.4 / 135.2 / 161.6 / 146.6 / 141.8），gh-ripgrep 167.5（180.1 / 167.5 / 164.1 / 166.7 / 167.7），gh-rust 146.2（146.2 / 168.7 / 136.3 / 156.8 / 136.3），mdn-div 34.4（45.4 / 33.3 / 36.7 / 33.3 / 34.4）；都 ≤ 200。中文维基用 Vector 2022 的 grid，首屏从 113 毫秒变成 146.6（grid 会算项的最小内容宽度，要读全文）；GitHub 页面没有明显变化（花时间的不是 flex/grid 外壳）。


## 第 2 层 · 2a/2b 审查修复（2026-10-17，提交 A：`Fix 2a/2b review findings`）

接着 2b-2（`61e832f`）。审查的复现页和脚本在会话临时目录的 `review2b/`；时间是 release、`qinglan-cli layout <页面> --test-font --time`，括号里是修之前。

### R1 网格自动放置没有计量（`layout/grid.rs`）

- 原来 `place` 里的 `Occ::free`/`set` 一格一格地查、一格一格地写，整个函数跑完才记一笔工作量，所以 `grid-auto-flow: dense`、跨很多列的项、几十万个项、1000 行的 `grid-row: span` 都能让排版跑几十秒而预算不起作用。
- 现在：占用图一次一个 64 位字（`word_mask`），`Occ::hit` 返回窗口里最后一个被占的格子（行、列），调用方直接跳过它（窗口里 `c..` 到这格的起点都会压到它，所以跳过是对的）；`place` 带一个 `limit`（页面预算还剩多少），每一步扫描、每一次 `set` 都记账（一步 1 单位，另外每 8 个字 1 单位），超了就停，没放的项 `ok = false`，调用方把 `exhausted` 置位（页面"不完整"）。
- **dense 打包不再是 O(n²)**：记 `first_free`（第一个还有空格的行，只往前走）；有定列的项再记每列的 `col_free`（该列第一个空格）。两者都是下界，所以从那里开始找不会漏掉位置。
- 跨轨道的项（`resolve_intrinsic`）：每项按 2 + 4×跨度 记账；查"有没有弹性轨道"原来对每个不同的跨度都把所有项扫一遍（跨度种类 × 项数 × 跨度），现在预先算一遍、按跨度分段只走一次；预算用完直接不做剩下的。
- 结果：h2a（dense，11.6 万项）超时 → 0.86 秒（不完整）；h2b（dense 加大跨度）超时 → 0.64（不完整）；h2c 超时 → 0.46；h2d（5 万个 1000×1000 的区域）6.4 → 0.70（不完整）；h2e（`auto-fill`，10 万×10 万）18.7 → 0.54（不完整）；k1_dense_1col 0.55 → 0.39（完整）；k1_dense_50000 4.2 → 0.38；k2_spanauto_10000 10.9 → 0.25（不完整）；v_gridrow 0.91 → 0.24（不完整，7 个项就能把预算吃光：每项跨 1000 行，窗口扫描本来就要 100 万次检查），debug 版 31.7 → 6.9 秒。

### R2 生成内容的盒子没有上限（`layout/boxtree.rs`、`css/cascade.rs`）

- 生成的盒子 + 文档节点数受 `MAX_NODES` 管：生成的盒子最多是"`MAX_NODES` 减去文档节点数"的 1/4（`Builder::gen_cap`），一个生成盒约 600 字节（盒、位置、排好的文字），所以一个页面不会因为每个元素都有 `::before`/`::after` 而多吃几倍内存；超了的 `::before`/`::after` 不生成，`BoxTree::cut` 置位，`LayoutResult::incomplete` 为真。
- 每个元素的 `::before`/`::after` 样式原来各算一份新的 `ComputedStyle`（每份约 1 KB：`BoxStyle` 和一串 `Rc`），25 万个元素就是 500 MB。现在和元素的样式一样共享：同一个父样式、同样的声明块（`ShareEntry`，共用 `share` 表），所以一整页的 `::before` 只有一份样式。
- 结果：n1_before_250k 和 n1_q_250k 的 `layout` 峰值 828 → 224 MB（提交内存），工作集 695 → 221 MB，2.1 → 0.9 秒。**没到 200 MB**：同一页去掉 `::before` 的孪生页（n1_plain_250k，25 万个 `<b>x</b>`，正好 50 万节点）本身就是 204 MB 工作集 / 207 MB 提交（`bench` 里 214 MB）——这是文档节点上限（50 万）自己的成本，不是生成内容的；剩下的 +17 MB 是每个元素一项的 `pseudo` 哈希表。要把 50 万节点的页面压到 200 MB 以下得缩小节点本身（`LayoutBox` 64 字节、`BoxOut` 32、DOM 72、显示列表 17 MB 等），不在这一轮。
- `hostile.rs` 里用 10 万个元素（20 万节点）：`::before` + `::after` 170 MB / 0.6 秒，10 万个 `<q>` 170 MB / 0.6 秒（修之前 `<q>` 290 MB）。

### R3 浮动放置没有计量（`layout/floats.rs`、`layout/inline.rs`）

- `FloatSpace::place` 每一轮（`band` + `next_bottom`）看一遍已有的浮动，现在每轮记 1 + 个数/4 单位，超过页面预算还剩的就停（浮动留在当前位置，页面"不完整"）；`avoid_floats` 同理；行内的 `line_band` 每次 1 + 个数/8。
- 另一处真正的二次方：`inline.rs` 里"这个浮动放过没有"用 `placed.contains(&k)`（Vec 线性查找），找浮动尺寸用 `fdims.iter().find`——10 万个浮动在同一个段落里就是 10 万 × 10 万。现在 `placed` 是按单位下标的 `Vec<bool>`，`fdims` 是按单位下标排好序的，用二分。
- 结果：h4_floats_noclear 7.2 → 0.67 秒（不完整）；k3_floats_30000/60000 0.27/0.40（不完整）；正常的浮动页（h4_floats、h4_floats_nested）0.33/0.69 秒不变。`tests/wpt` 浮动目录结果不变（`wpt --check` 通过）。

### R4 `documents_and_styles_jump_the_image_queue` 不稳（`tests/net.rs`）

- 原来 16 个同样慢（60 ms）的请求几乎同时结束，哪个先放出名额、线程先进哪个请求，是竞争。现在 `Fake::staggered`：16 个占满名额的请求里有一个只要 120 ms，其余 500 ms，所以先放出来的名额正好一个，那时队列里有 4 张晚到的图和 1 个样式表，样式表必须先开始；晚到的图要等 500 ms 那批结束。

### 新的恶意页面（`crates/qinglan-cli/tests/hostile.rs`，release，每个 ≤ 5 秒、≤ 200 MB）

`grid_dense_200k_items` 0.9 秒 / 89 MB、`grid_dense_wide_spans` 0.8 / 57、`grid_one_column_items_dense` 0.2 / 54、`grid_areas_1000_by_1000_lines` 0.9 / 79、`grid_10000_items_spanning_1000_auto_tracks` 0.4 / 18、`before_and_after_on_100k_elements` 0.6 / 170、`quotes_100k` 0.6 / 170、`floats_100k` 0.7 / 77。debug 版大小缩到 1/10。

## 第 2 层 · 2a/2b 审查修复（2026-10-17，提交 B：`Paint order of flex/grid items; body overflow to the viewport`）

### flex 和 grid 项的绘制顺序（`paint/build.rs`）

- css-flexbox-1 4.3、css-grid-1 9：项"和行内块一模一样地绘制"，只是用 `order` 排过的顺序。以前项当普通块走两趟（先所有块的背景，再所有行内内容），后一个项就盖不住前一个项的文字。
- 现在 `walk` 里遇到 `flex_item` 的孩子：第一趟（块的背景）跳过，第二趟（行内内容，CSS2 附录 E.2 第 7 步）用 `paint_box` 整个画（背景、它自己的浮动、内容一起，和行内块一样）。放在第二趟而不是第一趟，是因为 flex 容器本身是块级盒，它旁边的浮动（第 4 步）要盖在容器背景上、盖不住项的内容——`flexbox_flex-formatting-interop` 就是这样（第一次试放在第一趟，它退步了）。
- `z-index` 不为 `auto` 的静态项早就按 `position: relative` 算（2b-1），仍走层叠上下文的列表，不受影响。点击顺序没改。
- WPT：css-grid 554 → 564，css-flexbox 647 → 649，没有退步（`passing.txt` +12 条）。审查点名的 5 个绘制顺序测试里 2 个过了；`grid-item-text-background-not-interleaved-001` 和 `grid-layout-z-order-a/b` 还差（z-order 两个是静态项带 `z-index`，第一个差点在 (100,60)：参照是黄色，我们是白色，没逐个分析），`notes.txt` 里那几条的原因改了。

### `overflow` 从根元素或 `body` 传给视口（`layout/boxtree.rs`、`qinglan-app/src/page.rs`）

- CSS2 11.1.1、css-overflow-3 3.3：根元素的 `overflow` 给视口；根是 `html` 且它的值是 `visible` 时，用 `body` 的。视口上 `visible` 当 `auto`，`clip` 当 `hidden`；值传出去的那个元素自己按 `overflow: visible` 排版和绘制（不裁剪，也不是 BFC 的根）。
- 建盒子树时（`build_box_tree_with`）决定来源元素，只对它换一份 `overflow` 为 `visible` 的样式（`vp_source`）；结果记在 `BoxTree::viewport_overflow`。`Page::max_scroll` 在视口纵向是 `hidden` 时返回 0（`body { overflow: hidden }` 的页面在窗口里不能滚）。`auto`/`scroll` 照旧；横向没有滚动条，没变。
- WPT css-grid 里 6 个点名这件事的测试（`body { overflow: hidden }` 把超出的内容裁在 body 的盒子里）和绘制顺序的一起算在上面 +10 里。新测试 `overflow_of_the_root_or_body_goes_to_the_viewport`（`tests/grid.rs`）。

## 第 2 层 · 2a/2b 审查修复（2026-10-18，提交 C：只做了 `tests/wpt` 瘦身；首屏和体积两项没做）

### 做了：`tests/wpt` 去掉没人引用的文件

- 办法：从"我们跑的测试"出发（`passing.txt` 和各个 `RESULTS-*.md` 表里的全部测试，3602 个）顺着 `rel=match|mismatch`、`href`、`src`、`url()`、`@import` 找出所有被引用的文件；`README`、`LICENSE.md`、`fonts/`、`notes.txt`、`passing.txt`、`RESULTS-*.md` 一律留着；其余的删掉（521 个：被跳过的测试的支持文件 `css/CSS2/support`、`css/CSS2/reference`、`css/support/*testcommon.js`、没人用的图片和 `.ico`、`css-lists/parsing` 和 `crashtests` 里的东西）。
- 结果：文件 6356 → 5835；内容大小 9.73 MB → 9.19 MB；`du` 的磁盘占用 27.6 MB → 26.0 MB（NTFS 每个小文件占 4 KB 簇，6000 个小文件的"占用"比内容大得多）。**各目录通过数一个没变**（floats 76/100、floats-clear 172/211、generated-content 188/205、tables 219/250、zindex 29/29、flexbox 649/951、grid 564/1573、lists 80/143、tables 90/140；`wpt --check` 2067/2067）。
- **没到 20 MB**：剩下的大头是 `css/css-grid/grid-lanes`（masonry，不计划做，通过数 0）：磁盘占用约 7.4 MB、1800 个左右的小文件。删掉它们要改 css-grid 的总数（1573 → 约 940）和 RESULTS，不属于"没人引用的文件"，这一轮没动；要不要删由你定（删了就在 20 MB 以内）。

### 没做：grid 首屏时间、可执行文件体积

- zhwiki-rec 首屏 113 → 147 毫秒（grid 对所有项算最小内容宽度）：没改。想法：`resolve_intrinsic` 里单列项只对内在大小的轨道算；剩下的算（弹性轨道的 `find_fr`、flex 组、自动最小尺寸）才是省不掉的 `contribs`，要先量哪一处调用最多再动，下一轮做。
- 可执行文件：`qinglan.exe` 2 138 624 → **2 150 400**（+11 776：计量、`Occ` 的字操作、伪元素样式共享、视口 `overflow`、绘制顺序）；2b 的上限 2 027 000 已经超了 123 400，第 2 层总上限 2 516 582 还剩 366 182。链接映射（`/MAP`）和去重没做，下一轮做。

### 数字（release；**条件不合格**：`BatteryStatus = 1`，用电池；量的时候 qingpdf 会话的 cargo/rustc 在跑，进程数 0–3 个，不稳定）

- 首屏（`qinglan.exe --bench … --replay-delay 80 --width 1920 --height 1080`，5 次新进程，中位数）：zhwiki-rec 500.6 毫秒（791.6 / 553.0 / 490.5 / 500.6 / 498.4），gh-ripgrep 510.5（593.6 / 503.3 / 508.9 / 510.5 / 561.0），gh-rust 450.0（481.9 / 452.1 / 450.0 / 421.8 / 436.6）。**这些数字不能和 2b 的 113/170/149 比**（电池供电，CPU 降频，约 4 倍慢），要插电重量；2b 提交（`61e832f`）和这几个提交没有交替跑过。
- 内存（zhwiki-rec，滚到底的私有字节，5 次）：47 042 560 / 44 564 480 / 43 106 304 / 43 319 296 / 44 269 568，**最大 47.04 MB**（≤ 50，2b 是 47.86）。内存对降频不敏感，这个数可信。

## 第 2 层 · 2b-3 grid 之后的首屏和体积（2026-10-20）

起点 `24ce177`（`qinglan.exe` 2 150 400 字节；zhwiki-rec 首屏 113 → 147 毫秒）。目标：首屏回到 ≤ 120 毫秒（zhwiki-rec），exe 尽量回到 2b 预算 2 027 000，不删功能，排版结果不变。临时的计时代码只在暂存工作树里，没有提交。

### 1. 首屏：量出来的不是 grid 的最小内容宽度

- **分段计时**（`qinglan-cli bench`，zhwiki-rec，1920×1080）。2b-1（`ad58292`）：解析 21、样式 35、首屏排版 34（其中盒子树 6.5、预取文字 28、`layout_root` 0.7）、光栅 5，合计约 96。2b-2 之后：解析 22、样式 38.5、首屏排版 65（盒子树 7、预取 27、`layout_root` 28）、光栅 10，合计约 138。
- **原因**：2b-1 时 grid 还不认，维基的边栏堆在文章上面，`layout_root` 不到 1 毫秒就越过了停止线，首屏排版几乎是空的；grid 做对以后页面第一次排对，首屏真的有内容了（光栅 5 → 10，样式 +3.5：flex/grid 属性现在是有效声明）。**grid 的最小内容宽度不是原因**：这一页 `grid_widths` 调用 0 次，列方向的 `contribs` 只有 2 次、0.00 毫秒，行方向 14 次（量行高，带停止线）。所以没有为它加缓存或跳过。
- **真正的大头**：`.vector-dropdown-content`（语言菜单：`position:absolute; visibility:hidden; height:0; width:max-content`）的 shrink-to-fit 宽度要读它全部文字（约 190 个语言名，冷的字形整形），一次 19–21 毫秒（完整排版时已缓存，1.2 毫秒）。2b-1 时这一步被越过停止线躲掉了，完整排版里才付。
- **改动**（`layout/position.rs` 的 `layout_oof_with`、`mod.rs` 的 `deferred_oof`）：首屏优先（`stop_at` 有值）时，自己和所有后代的 `visibility` 都不是 `visible` 的脱离文档流盒子不排（看不见、点不到，也不挪动任何文字），记 `deferred_oof`，`LayoutResult::partial` 置位，所以完整排版一定会跟上并排它。有 `visibility: visible` 的后代就照常排。注意不能用 `stopped`：它会让后面的内容都被截掉。完整排版的结果不变。测试 `firstscreen::a_hidden_out_of_flow_box_waits_for_the_full_layout`；原有的"停止线以上逐项相等"测试（含中文维基）不变。
- **试过没采用**：预取文字的额度 `area/60` 换成 120、240、1000（环境变量临时扫了一遍）：首屏排版和总数都看不出差别（噪声 ±10 毫秒），没改。
- **结果**（`qinglan-cli bench`，旧 `24ce177` 和新交替跑，5–6 个干净轮次的最小值 / 中位数）：zhwiki-rec 首屏 124.0 / 143.8 → 109.3 / 120.5（−12% / −16%），首屏排版 60.1 / 62.1 → 39.7 / 40.7；gh-ripgrep 首屏 155.1 / 158.9 → 156.2 / 159.8（+1% / +1%）。

### 2. 体积

- **映射**（`/MAP` + v0 符号名，2a 末 `9e8fbf8` 对比现在，`.text`）：+200 KB（grid 排版 63 KB、`layout_block` +25.7 KB（flex/grid 的入口内联进去）、`parse_grid` 23 KB、`core::slice::sort` +22 KB、`parse_prop` +14.5 KB、`cascade` +12.5 KB）；`.rdata` +20 KB，`.pdata` +4 KB。
- **排序去重**：grid 里 3 处 `sort_by_key`/`sort_by`（每处一整份排序代码，共约 21 KB）改成 `sorting::sort_by_u64_key`（项目的跨度、`fr` 余数的小数部分按 `!f64_key`、基线用的第一项/最后一项改成 `min_by_key`/`max_by_key`，结果等于稳定排序的两端）。2 150 400 → 2 127 872（含上面的首屏改动）。
- **内联阈值扫描**（`RUSTFLAGS` 带 `+crt-static`，只是实验；最终写进 `.cargo/config.toml`）：`inline-threshold` 100 → 2 127 872；70 → 2 086 400；45 → 2 046 976；35 → 2 051 072（不单调）；45 加 `inlinehint-threshold=150`（默认 325，管 `#[inline]` 函数和泛型的 std 代码）→ **2 012 160**；45 加 150 换成 100 → 1 989 632（整页布局 +4%、解析 +10%，没采用）。最终：`-inline-threshold=45` 加 `-inlinehint-threshold=150`，**2 150 400 → 2 012 160（−138 240；2b 预算 2 027 000，余 14 840）**。`qinglan-cli.exe` 2 418 176 → 2 243 072。
- 没做：压缩 grid 算法本身的代码（`build_gx` 19 KB、`size_axis` 11 KB、`phase` 7 KB）、`.rdata` 的表。

### 3. 速度和内存（插电；这台机器上另有会话的 cargo 空转和零星的 qpdf，"干净"按 CPU 负载 ≤ 25% 算，数字有噪声）

- 整页布局 / 滚动，旧 → 新（`qinglan-cli bench`，zhwiki-rec 最小值 / 中位数）：整页布局三次测量 −0% / +6%、+10% / +6%、+7% / +1%（安静的那一轮，同一个二进制：+1% / +3%）；滚动中位数 +0% / +0%、+10% / +2%、+2% / +2%；gh-ripgrep 整页布局 −0% / +4%、+1% / +4%、+1% / +2%，滚动中位数 0%。zhwiki-rec 的整页布局最小值有一次到 +7–10%（那几轮旧程序自己也慢了 10%：噪声），**离 5% 的线很近**；若后面要严格守线，退路是去掉 `inlinehint-threshold`（只留 45：2 046 976，同一轮里整页布局 +1% / +2%）。
- `qinglan.exe --bench` 新进程各 5 次（旧 `24ce177` 和新交替；毫秒，内存十进制 MB）：

```
zhwiki-rec first-screen-ms            | old: med   175.4 max   337.9 [172.4 153.4 337.9 194.1 175.4] | new: med   129.6 max   304.3 [122.1 116.7 304.3 289.3 129.6]
zhwiki-rec peak-private-bytes         | old: med    45.2 max    45.7 [44.9 45.7 45.2 45.0 45.2] | new: med    45.4 max    45.9 [44.9 45.9 45.4 45.2 45.6]
zhwiki-rec private-bytes-after-scroll | old: med    43.8 max    44.3 [43.8 44.0 43.8 43.5 44.3] | new: med    43.9 max    45.9 [43.8 45.9 43.5 43.9 44.6]
gh-ripgrep first-screen-ms            | old: med   220.6 max   358.2 [160.5 358.2 352.5 220.6 164.6] | new: med   191.3 max   373.6 [152.4 373.6 365.0 191.3 182.2]
gh-ripgrep peak-private-bytes         | old: med    64.2 max    66.3 [63.1 66.3 64.2 64.8 62.6] | new: med    64.7 max    65.8 [64.7 65.8 65.8 64.6 61.4]
gh-ripgrep private-bytes-after-scroll | old: med    24.4 max    24.6 [23.7 24.1 24.6 24.4 24.5] | new: med    24.5 max    25.2 [23.9 23.7 25.2 25.0 24.5]
gh-rust    first-screen-ms            | old: med   181.1 max   324.6 [133.7 298.9 324.6 181.1 142.6] | new: med   173.7 max   308.7 [133.5 300.8 308.7 173.7 149.7]
gh-rust    peak-private-bytes         | old: med    64.5 max    65.8 [64.5 65.8 65.7 63.6 63.4] | new: med    63.3 max    68.0 [63.2 65.7 63.3 62.6 68.0]
gh-rust    private-bytes-after-scroll | old: med    23.3 max    24.9 [23.6 23.2 23.3 22.8 24.9] | new: med    24.2 max    26.6 [24.5 26.6 23.3 22.4 24.2]
```

- 测试：`cargo test --workspace --no-fail-fast` 328 条（原有）全过，另加 1 条（`firstscreen`），共 329；`qinglan-cli wpt --check` 1965 / 1965；clippy 无警告。


## 第 2 层 · 2c-1 路径光栅器、背景、渐变、圆角、边框样式、outline（2026-10-20）

代码：引擎新模块 `vector/`（`mod.rs` 路径和变换、`fill.rs` 覆盖率光栅和遮罩、`stroke.rs` 描边）；`css/bg.rs`（背景、渐变、圆角、outline 的值类型和解析）；`paint/gradient.rs`（渐变的解析结果和查表）、`paint/borders.rs`（圆角和各种样式的边框）；`paint/build.rs`、`paint/raster.rs`、`layout/boxtree.rs` 里相应的改动。测试：`vector` 单元 11 条、`paint::gradient` 6 条、`css::bg` 4 条、`tests/backgrounds.rs` 20 条（手算像素和面积）、`tests/paint.rs` 里两条旧边框测试按新行为改过。

### 路径光栅器（`vector/`，任务单 2c）

- **覆盖率**：按字体光栅器（stb_truetype v2、font-rs）的做法，每条边把它在每个像素里"右边的有符号面积"累加进缓冲区，再沿行累加得到覆盖率，所以抗锯齿是精确面积，不是采样。非零规则取绝对值截到 1，奇偶规则取三角波。缓冲区最多 4 MB（按行分带，一带不够就分几带），遮罩（`Mask`）最多 2048×2048 像素；窗口外的部分不算。同方向的形状在同一个像素里部分重叠时覆盖会略微偏多（这个方法的通病），整像素在内部的结果是精确的。
- **曲线**：二次、三次贝塞尔按容差切成线段（误差界 `0.25·dd/n²` 和 `0.75·dd/n²`，每条曲线最多 128 段）；圆角和椭圆用四段三次贝塞尔（常数 0.5522848）。边框和裁剪用的容差是 0.01 设备像素：0.1 时圆的切点处会浅 2–4%（用面积测试发现）。
- **描边**：每个线段一个矩形、每个转角一块楔形（斜接、圆、斜切；斜接上限按 SVG 的"斜接长度 / 线宽"）、每个开口端一个端点（平、圆、方），全部同一个绕向，再用非零规则填，就是并集。虚线先把折线切成段再各自描边；数量上限（多边形 13 万、虚线段 3 万）。输入里的非有限数一律丢掉。
- **变换**：`Transform`（2×3 仿射，`compose`、`apply`）和 `Path::transformed` 是 2c-2（CSS 变形）和 2e（SVG）的接口；路径可以先变换再填或描边。圆角矩形裁剪走专门的快路（见下），不走通用遮罩。

### 圆角和裁剪（`paint/`）

- **半径**（css-backgrounds-3 5.1、5.5）：百分比按边框盒算；任意一边上相邻两个半径之和超过边长时，所有半径按最小比例一起缩小；半径有一个分量是 0 的角是直角。背景的绘制区域取 `background-clip` 的盒子，半径用对应的内边缘（边框宽度、再加 padding 都从半径里减，最小 0）。
- **裁剪链**：`DisplayList::clips` 仍是矩形；`rounds` 与它平行，记每个裁剪自己的圆角形状和下一个有圆角的裁剪。`overflow` 的裁剪、背景的绘制区域都可以带圆角。光栅时圆角只和矩形差在四个角上：先照矩形画，再对触到的每个角盒，把像素按形状在那里的覆盖率从"之前"拉到"之后"（`之前 + (之后 − 之前)·覆盖率`，等于把这个元素的 alpha 乘上覆盖率再画），所以文字、图片、渐变、边框都一样处理，里面的大块区域完全不付钱。角盒的覆盖率每帧每个角只做一次。
- `<img>` 的 `border-radius` 把图片裁到圆角的内边缘（头像圆图是网页上最常见的用法）。

### 背景（css-backgrounds-3 2、3）

- **解析**：`background-image`（`url()`、四种渐变和 `repeating-` 形式）、`-position`（含 x、y 分开写和四值语法，边偏移折成"百分比加长度"：`right 10px` 是 100% 和 −10px）、`-size`（`cover`/`contain`/长度）、`-repeat`、`-origin`、`-clip`、`background` 简写（任意顺序、`/` 后接尺寸、颜色只能在最后一层、一个盒子值同时给 origin 和 clip）；多层最多 16 层，各列表按图片层数循环或截断。简写里没写的部分用 `Value::Ignored` 表示初始值（不分配内存，也不为只写了 `background: #fff` 的元素新建 `Backgrounds`：在 GitHub 页面上这一项值几 ms 和 2 MB）。
- `url()` 在解析时按样式表的地址（`<style>` 和 `style=` 按文档地址）变成绝对地址（`SheetCtx::base`、`ParseEnv::base`）；`var()` 展开出来的 `url()` 仍按原样、请求时按页面地址解析（和 `content: url()` 一样）。
- **图片请求**：盒子树里有 `url()` 背景的盒子，每层拿一个"文档节点数之后"的编号（和 `content: url()` 的图片同一个计数器，同样的样式每次排版编号相同），进 `LayoutResult::images`；图片头到了就记尺寸（`BgRef::nat`），所以宿主不用改：和 `<img>` 一样请求、探头、解码。头到达时整页重排（背景不占地方，但背景的瓦片大小要用自然尺寸；先不做"只重画"的优化）。图片按瓦片的设备像素大小解码（`ImagePlace::rect`），距离视口多远按 `ImagePlace::span`（背景的绘制区域的纵向范围），懒加载规则不变（离视口 ≤ 2 屏就请求，> 4 屏丢解码结果）。
- **瓦片**：尺寸按 `cover`/`contain`（图片有比例）、长度、`auto`（图片用自然大小，渐变用定位区域）；`round` 取整数张并拉伸（只有一个方向是 `round`、另一个方向 `auto` 时，另一个方向按图片比例变）；`space` 取整数张、间距平分、从边上开始，装不下两张就不重复；重复的瓦片至少 1 CSS px（Chromium 同；0.2px 的渐变瓦片就是一整块色）。第 k 张瓦片的左边取整到设备像素，宽度是解码时的整数宽度。
- **画布背景**：根元素（没有就 body）的背景图片也上画布（`canvas_background` 现在把"有背景图"算作有背景）：定位按根元素的盒子，绘制铺满视口和根盒较大的那个，重复照常。
- **记下的做法**：`background-attachment` 解析后丢掉（`fixed`、`local` 按 `scroll` 画，任务单）；`background-clip: text` 解析后按 `border-box` 画（做文字裁剪要一层离屏，没做）；`border-area` 不认；行内盒断行时每个片段各画一份背景图（不是 `box-decoration-break: slice` 的连续），圆角和边框只在真正的首、尾片段的外侧。

### 渐变（css-images-3 3、css-images-4 3）

- 线性（角度、`to` 边和角：50% 线过另外两个角，css-images-3 3.1.1）、径向（圆、椭圆；`closest/farthest-side/corner`、长度、`at <position>`）、锥形（`from`、`at`；位置可以是角度或百分比，角度按圈的百分比存；单位的 `0` 也认）、三种的 `repeating-`；前缀版本（`-webkit-linear-gradient`）和 `image-set()` 无效。只有一个颜色也有效（整块那个颜色，css-images-4 3.5）。
- **色标**：两个位置的色标拆成两个；颜色提示（hint）单独一项，只能在两个色标之间。修正（3.5.3）：第一个默认 0、最后一个默认 100%，位置不后退，没位置的一段平分；提示按 `t' = t^(ln 0.5 / ln H)`（`H` 是提示在前后两个色标之间的相对位置，css-images-4 3.5.4），幂用自己写的 `exp2(e·log2 t)` 短多项式（误差 < 2e-3，比 8 位通道的一级小；C 库的 `powf` 就 3.7 KB）。重复渐变的首尾位置相同时没有周期，取最后一个颜色。
- **插值**：所有颜色在预乘 alpha 的 sRGB 里线性插值（CSS Color 4 12.3：旧语法的颜色就是这样；引擎的颜色解析把 `lab()`、`oklch()` 等都转成 sRGB 了，所以只有这一个空间）。`in <颜色空间> [hue 方式]` 解析后丢掉（记下：`in oklab`、`longer hue` 等的 WPT 测试归 `limit`）。
- **画法**：每个渐变解析成几何（线性：起点和单位向量；径向：圆心和两个半径；锥形：圆心和起始角）加一张预乘 BGRA 的查表（线性和径向每 0.5 px 一格，最多 4096 格；锥形 1024 格），首尾之外用首尾颜色；画的时候每个像素算一次参数再查表。表里每格取格子中心的颜色，所以硬色标最多糊一格（0.5 px）。

### 边框、边框样式、outline（css-backgrounds-3 4、5；css-ui-3 6）

- 一般的实线矩形边框仍是几个矩形（和以前逐像素一样）；有半径、样式不是 `solid`、或者相邻两边颜色不同时做成一个 `BorderDraw`，光栅时按路径画。
- **转角连接**（4.3）：每条边只管它的楔形（边框盒外角到内角的四边形），颜色不同的两边在对角线上相接；同色的实线边合在一次填充里画（面积是相加的，所以对角线上没有缝）。有半径时是"圆环 ∩ 楔形"：圆环是外圆角矩形和内圆角矩形的奇偶填充，内半径 = 外半径 − 边框宽度（最小 0）。
- **样式**：`double`（宽度 ≥ 3 时外、内各三分之一，更窄当 solid）；`groove`/`ridge`（外半、内半各一半，≥ 2 px，明暗对调）；`inset`/`outset`（上、左和下、右明暗相反；明暗的算法照 Chromium：变暗 `v' = max(0, (v − 0.33)/v)`、变亮 `v' = min(1, v + 0.33)/v`，白变 `#ababab`、黑变 `#545454`）；`dashed`（线宽 ≥ 3 时划 2w、隔 w，否则划 3w、隔 2w，Chromium 的比例）、`dotted`（圆端点的点，直径 w、间距 w）；四边相同的虚线、点线沿边框中线画一整圈闭合路径，间距调整到整数个周期；四边不全相同时每条边沿自己的中线画直线，圆角那一段不画（记下），点线的第一个和最后一个点完整在边框盒里。`hidden`、`none` 宽度为 0（计算值里就是）。
- **outline**：宽度、样式（`auto` 按实线画，`hidden` 无效）、颜色（`invert` 当 `currentcolor`）、`outline-offset`；画在边框盒外面 offset 处，不占地方；有圆角时半径加 offset 和宽度；排到它所在的层叠上下文（`paint_box`）画完之后画（CSS2 附录 E.2 第 10 步），所以盖在后面的兄弟上面。

### 没做、留给后面

- `list-style-image`（属性本身还没有，所以渐变也不能用在它上面）和 `content: <gradient>`：没做；`content: url()` 的图片照旧。
- `background-clip: text`、`border-area`；`box-decoration-break: clone`；`image()`、`image-set()`、`cross-fade()`、`element()`、`border-image`、`mix-blend-mode`、`background-blend-mode`（任务单"不做"或不在第 2 层）。
- 渐变的 `in oklab` 等颜色空间；渐变里的 `calc()` 角度和无穷；容器查询单位；相对颜色。
- 图片到达时整页重排（背景图不改变版面，可以只重画；要做得改 `apply_image` 的 `keeps_box`）。
- 圆角下 `dashed`/`dotted` 的非四边相同情形、行内盒的 `slice` 连续背景、outline 的圆角只是半径加宽度的近似。
- 2c-2：`box-shadow`、`text-shadow`、`opacity`、2D 变形（含点击检测）。

### 验收数据（2026-10-20，release，插电 `BatteryStatus = 2`，没有别的 cargo/rustc/qpdf；机器上有 Claude 桌面版等后台程序，数字有噪声）

- **WPT**（`qinglan-cli wpt css/css-backgrounds css/css-images`；只放了不用 2c-2、2d、2e、不在计划里的特性的 reftest，没有 `<script>`，加上它们引用的文件；筛选脚本按这些特性的关键词筛，`css-backgrounds` 的 1007 个 reftest 里留了 212 个，`css-images` 留了 126 个；`tests/wpt` 磁盘占用 21 MB，真实字节约 7 MB）：
  - `css/css-backgrounds`：198 / 212（93.4%），bug 类 5；失败分类：bug (open) 5，limit (background-attachment) 5，limit (contain) 2，limit (gradient color spaces) 1，limit (relative colors, ::first-line) 1
  - `css/css-images`：58 / 126（46.0%），bug 类 0；失败分类：limit (calc in gradients) 5，limit (gradient color spaces) 18，limit (relative colors, ::first-line) 1，not planned 44
  - bug 类每条都有自己的一行在 `tests/wpt/notes.txt`：修掉的有单色标渐变、重复渐变首尾同位、`round` 另一方向保持比例、近零瓦片、`<img>` 圆角裁剪、锥形渐变里的 `0`；没修的 `bug (open)` 5 条（背景图在圆角边缘的重采样差、`background-origin: border-box` 加半透明边框的两条、行内盒断行加圆角的弧、嵌套 overflow 圆角裁剪的弧）写明了"没查"，没有谎称是别的。
  - `wpt --check`：wpt: 2217 of 2221 listed tests pass。
- **体积**：`qinglan.exe` 2,196,992 字节，比 2b 结束时的 2,012,160 多 184,832（任务单这一步 ≤ +60 KB，**超出约 125 KB**）。`/MAP` 对比 `.text`（+169 KB，当时）：`css::bg` +29 KB、`paint::raster` +19 KB、`paint::build` +16 KB、`vector::stroke` +8.5 KB、`vector::fill` +4 KB、`paint::borders` +5 KB、`css::cascade`（`compute_backgrounds`）+6 KB、drop glue +4 KB，C 库的 `powf`、`acosf`、`atan2f`、`sinf`、`cosf`、`fmodf` 约 +18 KB（自己写了 `pow`、弧的步长改用平方根、`fmod` 改用 `floor` 之后少了 7 KB，剩下的是 `atan2`、`sin`、`cos`，渐变角度和圆弧用得到）。第 2 层总上限 2,516,582 还剩 319,590 字节，2c-2、2d、2e 的原分配 350 KB 已经装不下：要先定怎么办（砍功能、放宽上限，或者再花一轮减体积）。
- **内存**（`qinglan.exe --bench`，zhwiki-rec 1920×1080，每次新进程）：滚到底后 Private Bytes 5 次 46.35、46.64、47.13、47.05、48.81 MB，**最大 48.81 MB**（≤ 50 MB；2b-3 是 43.5–45.9）。比旧程序多 2–3 MB 来自 `MiscStyle` 变大（每个样式多 `bg`、四个半径、outline）和规则里的背景值；简写只写颜色时不再分配之后，gh-ripgrep 滚到底 24.2–26.1 MB（和旧程序一样）。
- **速度**：滚动光栅（`render --full --scroll-bench`，1920×1080，dpr 1，中位数 / 95 分位）：zhwiki-rec 5.0 / 6.9 ms（673 帧，每帧 406 项），gh-ripgrep（圆角多）11.0 / 16.2 ms（109 帧，每帧 121 项、1415 个字形），gohugo 1.5 / 2.2 ms。**首屏**（`qinglan.exe --bench`，5 次新进程交替旧新）：zhwiki-rec 旧 145.1、新 151.3 ms（中位数）；gh-ripgrep 旧 185–216、新 205–230 ms（中位数，两轮之间旧程序自己也差 30 ms，机器当时有噪声），**新程序在 gh-ripgrep 上超过 200 ms 的目标，没能确认是不是噪声**，有一次 598 ms 的偶发慢启动（2-0 已知的 0.5 秒）。原因没查；怀疑是样式计算里 `MiscStyle` 变大和 `border-radius`/`background` 声明变多；要在安静的机器上重量。

## 第 2 层 · 2c-1b 修回归、gh-ripgrep 光栅、内存、体积（2026-10-20）

起点 `73c8318`。不删任何功能。

### 1. `wpt --check` 的三个退步：原因不是对角线转角

- `table-backgrounds-bc-cell-001`、`table-backgrounds-bc-table-001`、`col-paint-htb-rtl` 以前"通过"是因为**两边都画不出背景图和渐变，比较的是两张空白**；2c-1 之后画出来了，才露出两个真问题（折叠边框的表格边框不走 `BorderDraw`，对角线转角与它们无关）：
  1. **折叠边框模型里单元格和表格的 padding box**：背景图的原点（`background-origin: padding-box`）要从边框盒缩进它们在折叠边上分到的那一半（CSS2 17.6.2）。`LayoutResult::halves`（表格、单元格 → 上右下左四个半宽）由 `table.rs` 填，`paint/build.rs::backgrounds` 用它；行、列没有（它们在折叠模型里没有边框）。之前把整个边框盒当原点，贴图差 1–3 px。
  2. **`direction: rtl` 的表格列顺序**（CSS2 17.5.3）：表格从来没认过 `direction`，列和格子都从左排。现在 `table_inner` 里 `cx` 按反序累加，`span(c, n)` 给出一段列的 x 范围，格子和列/列组都用它（只做分隔边框模型；折叠边框的边没有镜像，记在这里，属于 2d 的 rtl 一起做）。
- 测试：`tests/backgrounds.rs` 两条手算（折叠单元格背景从缩进的 padding box 排：x 2..26 的盒子、4..24 的瓦片，硬色标在 14；rtl 表格里先写的 30px 红格在右边 50..80）。`wpt --check` 2221 / 2221。

### 2. gh-ripgrep 首屏和滚动：光栅慢了 2.6 倍，修了大半

- **量法**：`qinglan-cli bench`（1920×1080）旧 `5fab466`（暂存工作树、自己的 `--target-dir`）和新交替，5 个干净轮次（插电；每轮前后都没有 cargo/rustc/qpdf/link/cl 在跑，这次机器是安静的；`/tmp` 里的脚本不入库）。
- **找到的**：2c-1 的滚动光栅 gh-ripgrep 中位数 4.7 → **12.4 ms**（首屏光栅 7.5 → 16–19）。原因是**圆角边框的圆环按整个盒子的面积填**：`fill_polys` 的累加缓冲和逐行累加和是 O(宽 × 高)，GitHub 页上的大盒子（文件列表、侧栏卡片）一帧要填几十万格。`paint/borders.rs::fill_ring`：圆环只可能出现在四条带里（上、下两条是圆角占的行，中间左右各一条窄带），各填各的、带之间不重叠（不会重复混合），所以大盒子的代价是周长而不是面积；小盒子（< 16384 像素）照旧一次填。`ring_tests` 比较带状填法和整块填法逐字节相同（5 种尺寸、半径、边框宽度）。
- 圆角裁剪的角盒覆盖率（`corner_mask`）以前每帧每个角重新栅格化一遍整个圆角矩形的多边形；它只取决于这个角的两个半径和角盒大小，所以放进线程局部的缓存（最多 512 项，角盒 ≤ 64 像素）跨帧复用，缓存里的遮罩按"盒子左上角"为原点。这一项单独量过：对 gh-ripgrep 几乎没有影响（光栅 18.7 → 16.0），圆环才是大头；保留它是因为滚动时每帧省一次几何。
- `decor`：只有纯色、方角、`background-clip: border-box` 的背景直接画一个矩形，不走 `backgrounds`（`halves` 的哈希查找也只在折叠表格里做）。
- **结果**（5 轮，gh-ripgrep，旧 → 新，中位数 / 最小值，毫秒）：解析 4.6 → 4.9；样式 171.0 → 177.0（+3.5%，噪声：最小值 153.3 → 159.7）；首屏排版 50.3 → 47.7；显示列表 0.9 → 2.1；光栅 7.0 → 11.7；`bench` 首屏 228.1 → 249.1（最小值 216.5 → 221.1）；滚动中位数 5.1 → 5.4（+5.9%，最小值 +13.6%），95 分位 6.6 → 8.8。zhwiki-rec：首屏 174.0 → 182.6（最小值 159.8 → 145.1，噪声大），滚动中位数 6.3 → 6.2。`qinglan.exe --bench`（各 5 次新进程，交替）gh-ripgrep 首屏 旧 227.9 / 196.9 / 238.3 / 710.2 / 323.7、新 261.6 / 218.7 / 574.4 / 704.1 / 257.9（两边都有 0.5–0.7 秒的偶发慢启动，2-0 已知）；滚动中位数 5.9 → 6.5。
- **目标没达到**：gh-ripgrep 首屏 ≤ 200 ms 做不到——这台机器上旧程序自己就是 197–238 ms，样式阶段一个就 150–180 ms，和 2c-1 无关；2c-1 多出来的是光栅 +4.7 ms 和显示列表 +1.2 ms（首屏冷的圆环、角盒遮罩；显示列表里 `BorderDraw`、圆角裁剪的记账，没有再查）。滚动 gh-ripgrep 仍比旧程序慢约 6–14%（95 分位 +30%，8.8 ms，远低于 16 / 33 ms 的线），没压进 5%。

### 3. 内存

- `MiscStyle` 每个"声明过杂项属性"的元素一份（~25 000 份），2c-1 之后每份从 ~96 字节变成 ~256（四个半径 `[(Lp, Lp); 4]` 128 字节、`Outline` 24、`bg` 8）。现在 `radius: Option<Rc<[(Lp, Lp); 4]>>`、`outline: Option<Rc<Outline>>`（没写就是 `None`，`MiscStyle::outline()` 给初始值）；计算结果和初始杂项样式相等时（`vertical-align: baseline`、`background: none` 之类）直接共用那一份。
- zhwiki-rec 滚到底私有字节（`qinglan.exe --bench`，5 次新进程）：旧 `5fab466` 43.45 – 45.38；2c-1（`81c1964`）46.94 / 46.96 / 46.35–48.81；现在 45.81、46.41、46.58、**47.98**、45.83（最大 47.98，目标 ≤ 47 **没达到**；只省了约 0.5 MB，`MiscStyle` 不是主要部分）。
- **没查清**：峰值私有字节 旧 45.7 → 56.3 – 56.9（2c-1 就是这样，不是这一步引入的），滚动中途有一个约 10 MB 的临时峰（怀疑 `fill_polys` 的累加缓冲，最多 4 MB，和圆角/渐变的遮罩）；滚到底之后回落，但比旧程序多留 1.5–2.5 MB。留给下一步：用 `--features mem-stats` 在滚动中取样。

### 4. 体积

- `fmath.rs`（新）：自己的 `sin_cos`、`pow`、`log2`、`atan2`，替换 C 库的 `sin`/`cos`/`pow`/`atan2f`/`logf`（渐变角度和锥形渐变、圆弧、`lab()`/`oklch()` 的色相和伽马）。映射里这些加起来约 25 KB（`pow` 6 KB、`sin`/`cos`/`_sincos_special`/`__remainder_piby2*` 约 7 KB、`atan2f` 4.7 KB、`sinf`/`cosf`/`logf` 约 5 KB）。测试对照 C 库（误差 < 1e-11、2e-6、1e-9 相对）。
- `qinglan.exe`：2,196,992（`73c8318`）→ 修回归后 2,199,040（+2,048）→ 去掉 C 库数学、`MiscStyle` 瘦身后 2,179,584 → 加圆环分带和角盒缓存、纯色快路后 **2,183,168**（比 `73c8318` 少 13,824；2c-1 的预算 2,073,600 仍超 109,568，留给后面的步骤扣）。
- 没做（调用次数用完）：`css::bg` 的背景长属性共用一个解析器、四种渐变共用代码、`paint::raster::draw` 里的 `bg` 瓦片循环做成非泛型的函数、`hypotf` 320 字节、`fmod` 704 字节。`/MAP` 里 2c-1 新增的大头仍是 `Ctx::draw` 20.6 KB、`decor` 14.2 KB、`parse_background` 6.8 KB、`parse_image` 5.9 KB、`stroke_polys` 3.2 KB。

## 第 2 层 · 2c-2 阴影、透明度、2D 变形（2026-10-20）

起点 `72b0a79`。代码：`css/fx.rs`（解析、`Fx`、变换矩阵）、`paint/fx.rs`（模糊、阴影、组的光栅）、`paint/build.rs`（阴影项、组、行内透明度）、`paint/raster.rs`、`layout/mod.rs`（层叠上下文、包含块）、`interact.rs`（点击检测）、`vector/mod.rs`（`Transform::inverse`）。测试：`tests/effects.rs` 29 条（手算像素），WPT 见下。

### 样式
- 新属性 `opacity`、`box-shadow`、`text-shadow`（继承）、`transform`、`transform-origin`（`PropId` 增加 5 个，共 119，还在 u128 位图里）。前四个除 `text-shadow` 外放进 `MiscStyle::fx: Option<Rc<Fx>>`（全是初始值时是 `None`，每个元素只多 8 字节）；`text-shadow` 在 `TextStyle::shadow`。`opacity` 存成 0..=255（四舍五入，0.5 → 128，和 Chromium 一样）。
- `box-shadow`/`text-shadow` 最多 16 个；长度在计算时变成 px。`transform` 函数：`matrix`、`translate(X/Y)`、`scale(X/Y)`（数或百分比）、`rotate`、`skew(X/Y)`；**3D 函数只取 2D 部分**（记在这里）：`matrix3d` 取 m11 m12 m21 m22 m41 m42，`translate3d`/`translateZ`、`scale3d`/`scaleZ`、`rotate3d`/`rotateX`/`rotateY`/`rotateZ`（取旋转矩阵左上 2×2，所以 `rotateX(a)` 是 `scaleY(cos a)`）、`perspective()` 当恒等；**z 方向缩放为 0 时矩阵奇异，整个元素不画**（WPT `transform3d-scale-004` 的要求）。`translate`、`rotate`、`scale` 这三个单独的属性（css-transforms-2）没做，`transform-style`、`perspective` 属性没做。`transform-origin` 只用前两个值（第三个只检查是长度）。
- 层叠：`opacity` < 1 或有 `transform` 造层叠上下文（`makes_context`）；不是定位盒的，和 `z-index: 0` 的定位盒一起、按文档顺序画（css-color-4 12.1，css-transforms-1 2；`paint_order` 把它们放进 `stacked`，`z` 当 0，`position` 记成 `relative`）。有 `transform` 的盒子是绝对定位和 fixed 子孙的包含块（`makes_cb`；块、flex、grid、表格盒用它；`Layouter::fixed_cb()` 在包含块栈里找最近的变换祖先）；**表格的行、行组、标题不是**（WPT 5 条 `limit (table parts)`）。画的时候 `in_fixed` 遇到变换祖先就不算 fixed。

### 阴影（css-backgrounds-3 6.1）
- **形状**：外阴影是边框盒 + 偏移 + 扩展（宽高不小于 0），圆角半径按 6.1.1：半径 + 扩展，半径小于扩展时扩展先乘 `1 + (r/s − 1)³`；内阴影是 padding box 里“洞”的补集（洞 = padding box 缩小扩展量、再偏移，半径同样处理）。外阴影画在背景下面、只画在边框盒外面（边框盒的形状从阴影里挖掉，圆角用已有的角遮罩）；内阴影画在背景上面、边框下面，只画在 padding box 里。空盒子（0×0）仍有外阴影；多个阴影第一个在最上面。行内盒每个片段各画一份。
- **模糊**：标准差 = 模糊半径 / 2（6.1.2）。三次盒式模糊，盒子大小按 filter-effects-1 `feGaussianBlur` 的公式（`d = floor(σ·3·√(2π)/4 + 0.5)`，偶数时两次 `d` 一左一右、一次 `d+1`），每次是滑动和，代价和半径无关。阴影窗口是形状外扩 3 次盒子的伸展量；窗口超过 4M 像素不画（整页大的带模糊阴影会没有）。
- **缓存**：模糊后的遮罩按（形状宽高、半径、σ、内阴影的洞）哈希，放进线程局部的缓存，每条最多 1M 像素、总共 6 MB，满了清空；滚动只平移整数像素，所以不重新模糊。没有模糊、角是方的外阴影（`0 0 0 5px` 之类，还有 `100vmax` 的遮罩）不用遮罩，直接按矩形画。逐行画的时候边框盒里面的整段直接跳过。
- **text-shadow**：每个阴影一个文字项（偏移、颜色、模糊），画在文字下面，最后一个在最底下；文字透明时阴影仍画（`color: transparent`）；**下划线等装饰线没有阴影**（WPT 一条 `bug (open)`）。有模糊的文字阴影把整个 run 的字形覆盖合到一张遮罩上再模糊，**每帧重新模糊，没缓存**（合成的 36 张卡片页上 72 个 run 约 15 ms）。

### 透明度和变形的画法：组
- **显示列表**：有 `opacity` < 1 或 `transform` 的盒子（`paint_box`）把自己画进一个 `Group`（`DisplayList::groups`），在父列表里是一个 `ItemKind::Group` 项，`bounds` 是组内容的外包框经变换后的外包框、`clip` 是组外面的裁剪；组内的项仍用页面坐标（没变换之前），裁剪链从头开始（`Builder::sinks`）。所以索引、可滚动高度（`DisplayList::height`）自动包含变换后的范围（只向下；水平方向本来就没有横向滚动）。`opacity: 0` 和奇异变换不生成组。
- **行内元素的 `opacity`**：行内盒（`<span>`、`<a>`）的片段按连续的一段成组，跨行共用一个组（同一个元素的两行重叠时只合成一次，WPT `t32-opacity-offscreen-multiple-boxes`）；只含它的行内内容，**里面的浮动和块不在组里**（`limit (inline opacity)`）。行内元素的 `transform` 不生效（规范如此）。
- **光栅**（`paint/fx.rs::group`）：先把变换分成“缩放 k（两列长度的较大者，限制在 1/8..8）”和“余下的线性部分 R”。组内的项按 `dpr·k` **直立地**画（文字按缩放后的字号重新光栅，所以放大不糊），图片仍按页面的解码大小去要（`Ctx::base_dpr`）。R 是单位矩阵时只剩整数像素的平移：不透明度 255 就直接画进帧（偏移取整，最常见的 `translate(-50%, -50%)`、`scale(1.05)` 没有额外代价）；不透明度 < 1 画进一张和可见部分一样大的层再按 alpha 叠上去。R 不是单位矩阵（旋转、斜切、翻转）：算出目标区域在层空间里对应的范围，画进那一块层，再逐像素逆映射、双线性取样叠上去（四分之一圈、翻转时平移取整，取样落在像素中心，所以是精确的）。**记下的选择**：旋转和斜切的边缘比精确覆盖率糊（取样 vs. 路径覆盖），WPT `transforms-skewX/Y`、`transform-compound-001` 因此不过（`limit (resampled layers)`）；文字是“直立画好再变换”，不是转成路径。
- **上限**：层只分配组的可见部分，所以不超过帧的大小（帧是视口或一条视口带）；旋转层的源范围上限是帧像素数的 2 倍；嵌套的组最多 6 层。**超过上限的组整个不画**（不是“去掉透明度照画”：旋转的组去掉变换会画在错的地方；不透明度的组碰到上限只有嵌套过深一种情况）。这一点和任务单的写法（“不做透明直接画”）不同，记在这里。
- **点击检测**（`interact.rs`）：`PageIndex` 对每个变换盒（含祖先）存逆矩阵，`link_at` 先把指针经逆矩阵映回盒子自己的坐标再比较片段矩形；奇异变换的逆是 NaN，什么也点不中。`opacity: 0` 的元素仍可点（和浏览器一样）。

### WPT（`qinglan-cli wpt`，提交 3bcf9ec；新目录用 `tests/wpt/` 里同样的筛法：有 `rel=match|mismatch`、没有 `<script>`、没有 3D、`filter`/`mask`/`clip-path`/动画/字体/书写方向/SVG/表单……的页面）
- `css/css-transforms`：133 / 146（91.1%），bug 类 4（`bug (open)`，每条有原因：`transform-display-002/003`、`transform-generated-001`、`transform-transformed-tr-percent-height-child`，都没查）；其余 9 条失败是 `limit`：`transforms-skewX/Y`、`transform-compound-001`（重采样）、5 条表格部分不是包含块、1 条手动（hover）测试。
- `css/css-color`（`opacity`）：11 / 12（91.7%），bug 类 0（`inline-opacity-float-child`：行内元素里的浮动不在组里，`limit`）。
- `css/css-text-decor/text-shadow`：16 / 18（88.9%），bug 类 1（`decorations-multiple-zorder`，`bug (open)`）；1 条是彩色 emoji（2d）。
- `css/css-backgrounds`（加了 25 个 `box-shadow` 测试）：220 / 237（92.8%），bug 类 6 = 原来 5 条 `bug (open)` + `box-shadow-overlapping-001`（行内盒的阴影和背景一起画在行里文字之前，盖不住前一个兄弟的文字，`bug (open)`）；`box-shadow-border-radius-001`（csswg-drafts#7103，规范后来改了，`limit`）、`box-shadow-radius-000`（模糊差 ≤ 5/255，`limit`）。
- `css/css-images` 不变 58 / 126。`wpt --check`：2403 / 2403（原 2221，加了 182 条）。`tests/wpt` 22 MB（≤ 24 MB）。测试页挑选脚本不入库（`/tmp` 里的）；`dynamic-blur`（`onload` 里改样式）手动去掉了。`classify` 里的“2c-2”一类删了；没做的 `translate:`/`rotate:`/`scale:`、`perspective`、`preserve-3d` 归 `none`（不计划）。

### 体积
- `qinglan.exe`：2,183,168（2c-1b）→ **2,253,824**（+70,656），任务单这一步的预算 2,244,608（+61,440），**超出 9,216 字节**。第 2 层总上限 2,726,297 还剩 472,473。链接映射（`/MAP`，v0 名字，按模块累加）里这一步新增的大头：`css::fx` 约 17 KB（`parse_transform` 6.3、`parse_shadows` 2.1、`Fx` 的 `eq` 1.5）、`Ctx::draw` 20.8 KB（把 `shadow`、`group`、`text_blur` 内联了进去）、`Builder::decor` 16.8 KB 加 `box_shadows` 3.0、`blur_mask` 2.8、`finish_group` 2.0、`Builder::text` 3.3。已经做的：新类型的 `Debug` 只写名字、冷函数 `#[inline(never)]`（这两项没有改变文件大小，LTO 本来就去掉了没用到的派生）。没有再做的：`parse_transform` 改成表驱动、`group` 的两条合成路径合并、去掉 `matrix3d` 等的专门解析。按任务单“超了就停下报告，不单独开压体积的一轮”处理，留给第 2 层最后统一压。

### 速度和内存（release，插电 `BatteryStatus = 2`，CPU 占用约 28%，**机器上有一个 Emulator.exe 在忙（约 0.7 个核）**，没有 cargo/rustc/qpdf/link；`qinglan.exe --bench` 1920×1080 dpr 1，每次新进程）
- zhwiki-rec，5 次：滚到底后私有字节 47.14、46.87、46.62、47.33、46.42 MB（**最大 47.33 MB**，≤ 50）；首屏 172.0、164.4、141.0、153.6、166.3 ms（中位数 164.4，≤ 200）；滚动每帧中位数 6.6–6.8 ms、95 分位 10.5–12.4 ms（≤ 16 / 33）。
- gh-ripgrep（阴影、变换、透明度声明最多的一页），5 次：首屏 223.7、237.3、190.7、232.3、266.0 ms（中位数 232.3，**仍超 200 ms**，2c-1/2c-1b 就没达到，这一步没有改善也没有明显变坏；样式阶段占大头）；滚动中位数 5.7–6.3 ms、95 分位 10.4–13.5 ms；滚动光栅（`render --scroll-bench`）中位数 4.0 / 4.3 ms，95 分位 6.7 / 7.6 ms；内存 24.9–26.3 MB。gh-rust 3 次：首屏 172–198 ms，滚动中位数 6.4–8.4 ms、95 分位 11.0–17.9 ms。
- **压力页**（合成的，300 张绝对定位卡片：圆角、三层阴影含 14px 模糊和内阴影、带模糊的 text-shadow，三分之一有 `opacity`，五分之一旋转，七分之一缩放；视口里同时约 36 张）：滚动光栅中位数 73 ms（95 分位 89）。分项（中位数）：只有三层阴影 34.5 → **17.0 ms**（模糊遮罩缓存加按行跳过边框盒内部之后；其中约 5 ms 是页面本身），加模糊文字阴影 32.7，只有透明度 8.5，只有旋转和缩放 16.7。**这页不达标（≤ 16 ms）**：每帧要混 36×3 个阴影窗口和 72 个没缓存的模糊文字 run；真实页面（GitHub、MDN、Discourse）一屏上没有这么多，留给后面（文字阴影缓存、阴影只混可见的行带）。
- 备注：速度和内存是用 `2,253,312` 字节的那一版量的；之后只改了阴影的逐行循环（压力页 34.5 → 17.0 ms，与 zhwiki-rec 无关），最终是 2,253,824。最后一次完整检查：`cargo test --workspace --no-fail-fast`: 403 通过、0 失败；`cargo clippy --workspace --all-targets` 无警告；`qinglan-cli wpt --check` 2403 / 2403。


## 第 2 层 · 2c-3 样式速度、超过层数上限的组、文字阴影缓存（2026-10-20）

起点 `6536604`。代码：`css/tokenizer.rs`（按字节、跳读模式）、`css/scan.rs`（新，先读规则）、`css/syntax.rs`（`CvBuilder`）、`css/stylesheet.rs`、`css/facts.rs`、`css/mod.rs`、`paint/fx.rs`。测试：`css/scan.rs`（和 `parse_rule_list` 逐项对比）、`tokenizer.rs`、`tests/effects.rs`（+4 条，共 33）。

### 1. 样式速度：时间花在哪里（gh-ripgrep，1920×1080，`qinglan-cli style --time`，27 张样式表、约 2.5 MB、21782 条规则里 2461 条留下）

- **之前**：`parse_sheets` 115–140 ms，是样式阶段（约 170 ms）的大头；`index_rules` 1.4，级联 32–38（`match_plain` 17.9、`compute` 9.6；24xx 个元素里 1941 次命中匹配缓存）。`parse_sheets` 里：分词加建组件值（4 个线程，墙钟 56–69 ms）、转成规则 62 ms（其中选择器解析 20、`facts` 判断 5、声明解析 10，剩下约 27 ms 是丢掉被淘汰规则的组件值树）。留下的规则一共只有 13222 条声明，所以**自定义属性的继承和 `var()` 不是瓶颈**（`compute` 9.6 ms 里包括了），选择器匹配也不是：时间在“把 90% 马上要丢掉的规则完整分词、建树、再释放”。
- **做了什么**（改动按收益排）：
  1. **先读规则，不给用不上的东西分词**（`css/scan.rs`）。一条样式规则只读到它的 `{}`：块用分词器的**跳读模式**（`Lexer::skim`，同一套分词，只是不留文字、不分配，所以块在哪里结束和整段分词是一样的）只找到末尾，记成字节范围；留下的规则才由 `stylesheet.rs` 对这段范围分词、建树（`component_values_at`，嵌套上限从外层块数起，和整页一次建树一样）。`@media`、`@supports`、`@layer` 的块读成规则列表（不适用的 `@media` 不读），其余 at 规则只跳读。
  2. **选择器先在读的时候筛一遍**：前缀（选择器）也先跳读，每个选择器顶层的 `.类`、`#id`，文档里没有的就记下；一条规则的所有选择器都有这样的类或 id，就当场丢掉，不对它的选择器建组件值（`facts::lacks_class/lacks_id`，不看括号和方括号里的、带转义的、名字前有注释的、怪异模式；只会比 `DocFacts::possible` 少丢，不会多丢，有测试对比）。读的工作在 4 个线程里按样式表分，选择器解析和 `facts` 判断（20 + 5 ms）因此大半不在主线程。
  3. **分词器按字节**（原来先把全文变成 `Vec<char>`，4 倍内存）：非 ASCII 的每个字节都是名字字符，所以只有造字符串、转义的地方要管字符；输入预处理（CR、FF、NUL）只在文本里有它们时才复制（`normalize`）。
- **之后**：`scan_sheets` 25–31 ms（4 线程墙钟，最大的一张 721 KB 的样式表是下限）+ `parse_sheets` 21–24 ms（主线程）；`index_rules` 0.9；级联 22–27；**样式阶段合计约 167 → 80 ms**。gh-rust：`parse_sheets` 122 → scan 25 + parse 24。
- **验证**：49 页 `qinglan-cli style`（1920×1080）转储前后**逐字节相同**（`cmp`，脚本把两份放在暂存目录里比）；`css/scan.rs` 的测试把 `scan_normalized` 的结果和 `parse_rule_list(parse_cvs(..))` 逐条比（前缀、块内容、嵌套的规则列表、深嵌套的上限、CR/NUL、反斜杠、`url(`、字符串里的括号、没有闭合的块、第一个值是 `{}` 块的怪规则等）；跳读和正常分词的词界一样（`skimming_finds_the_same_token_boundaries`）。
- **没做 / 留着**：块在 `@layer` 里时要读两遍（先跳读找末尾，再读里面的规则；约 5–7 ms，一张 721 KB 的样式表就是一个 `@layer`）；跳读每字节约 10 ns，按字节专写一个快路可以到 1–2 ns（要先对照分词器做模糊测试）；单张样式表不能分到几个线程；级联（22–27 ms，`match_plain` 约 18）没动。

### 2. 超过层数上限的组（`paint/fx.rs::group`）

- **不再丢**。深度上限 6（每层一张离屏层）以外的组**直接画进当前画面**：不透明度去掉（记在这里：嵌套多层半透明的外圈会比真实的更实）；缩放和平移（`same`）仍然生效，旋转、斜切、翻转去掉，内容画在排版的位置。再深到 64 层（`MAX_NESTED`，画的时候每层递归一次）才不画，对应一个叠了 64 层以上特效的页面。
- **旋转层太大**（层要的像素超过 `max(帧, 512)² × 2`，比如整屏大小的元素转 45° 以上、或大角度斜切）：目的地的行**分带**画，每带一张自己的层，带高从整个区域开始、放不下就减半，直到放得下（一行也放不下的才跳过那一行，这只发生在斜切到几乎 90° 且元素大得不合理时）。像素和原来一张大层画的相同（同样的取样），只是每带把组里的项画一遍。这比“每个项各自变换”准：文字和图片没有逐项变换的办法（画路径的 `Transform` 只对路径用）。
- 测试（`tests/effects.rs`，修之前都失败）：8 层嵌套的 `opacity: .9` 里一个红框（前 6 层有层：`.9^6`，后 2 层直接画，红色在，值 [118,118,255] ± 8）；第 7 层 `scale(2)`、第 8 层 `rotate(90deg)`（缩放保留、旋转去掉）；70 层嵌套（超过 64）不崩、不卡；`left:-12000px; width:12100px` 的元素 `skewX(89.5deg)`（要 1.15M 像素的层）在 100×100 里整个画出来，带 `opacity:.5` 也是。

### 3. 文字阴影的模糊缓存（`paint/fx.rs::run_mask`、`blend_mask`）

- 带模糊的文字阴影改成**整个 run 的字形覆盖模糊一次、留着**（和盒阴影的遮罩共用 `MASKS`，上限 6 MB、每条 ≤ 1M 像素）。键：字形表（字体、字形、`advance`、`dx`、`dy`）、字号、`dpr`、σ、起笔位置的小数部分（滚动只移整数像素，所以同一个 run 滚动时命中）。遮罩带着相对笔位的左上角和大小存在前 16 字节。一个 run 的遮罩超过 1M 像素（或没有字形）就走原来的路：只模糊进入裁剪区的字形，每帧重做。
- 测试：模糊文字阴影滚动三帧，第三帧（缓存命中）和另一个线程（缓存是线程局部的）从头画的那一帧逐字节相同，而且是上一帧上移十行。
- **合成压力页**（重建的，`gen_stress2.py` 在暂存目录里：300 张绝对定位的卡片，圆角、三层盒阴影含 14 px 模糊和内阴影、两层模糊文字阴影，三分之一 `opacity: .8`，五分之一 `rotate(3deg)`，七分之一 `scale(1.05)`；视口里约 36 张；`render --scroll-bench`，第二遍（热），1920×1080，机器上 CPU 负载 50–60%，所以绝对值偏大，要看旧新对比）：只有卡片 6.9 / 7.1 ms（旧 / 新）；加盒阴影 47.3 / 43.7；加文字阴影 **33.9 / 13.8**；加透明度 17.3 / 17.1；加旋转缩放 35.9 / 32.8；全部 131 / 114。**≤ 16 ms 没有达到**：文字阴影从 +27 降到 +7 ms，但盒阴影（+37 ms，108 个阴影窗口、每帧约 5M 像素逐个混）、旋转缩放（+26）和透明度（+10）每帧还是要重画。要到 16 ms 得缓存“带阴影的整张卡片”（把阴影和卡片画成一张位图、滚动时平移），这是后面的事。

### 4. 数字

- **首屏**（`qinglan.exe --bench`，每次新进程，1920×1080，旧 = `6536604` 的 exe 拷贝，新 = 本步，交替跑）：
  - 插电（`BatteryStatus = 2`，有两个 Emulator.exe 每个约 0.25 个核，CPU 负载 26%）：gh-ripgrep 5 轮 旧 191.7 / 219.9 / 197.6 / 201.4 / 213.6，中位数 **201.4**；新 158.4 / 134.6 / 175.3 / 138.3 / 126.8，中位数 **138.3**。gh-rust 3 轮 旧 182.3 / 180.3 / 179.1；新 106.0 / 131.3 / 137.9（中位数 131.3）。zhwiki-rec 3 轮 旧 262.9 / 159.7 / 168.8；新 151.9 / 154.2 / 149.3（中位数 151.9，没有变慢）。
  - 收尾时电脑变成用电池（`BatteryStatus = 1`，负载 40–66%），这一轮**只能当旧新对比，不是验收数字**：gh-ripgrep 旧 400.2 → 新 281.8；gh-rust 328.3 → 233.4；zhwiki-rec 355.6 → 306.5（5 轮中位数，交替跑）。
- **内存**（滚到底后私有字节，5 轮最大值）：zhwiki-rec **47.11 MB**（旧 50.63，用电池那一轮；之前插电的旧值 47.3）；gh-ripgrep 23.27；gh-rust 21.13。滚动帧中位数 zhwiki-rec 12.1（电池，旧 12.5）、gh-ripgrep 10.7（旧 11.2）、gh-rust 14.0（旧 14.4）。
- **体积**：`qinglan.exe` 2,253,824 → **2,268,672**（+14,848，预算 +20,480）。没有生成链接映射（`/MAP` 要改 `RUSTFLAGS`，这一步不许）；体积直接按文件字节量。
- `cargo test --workspace --no-fail-fast`：412 通过（效果测试 33 里有一条的测试自己在调试版里 `u8` 溢出，改了再跑过）；`qinglan-cli wpt --check` 2403 / 2403；`cargo clippy --workspace --all-targets` 无警告。

## 第 2 层 · 2d-1 网页字体（2026-10-20）

代码：引擎 `fonts/`（`bits.rs`、`huff.rs`、`inflate.rs`、`brotli.rs`、`brotli_tables.rs`（生成）、`brotli_dict.z`、`sfnt.rs`、`woff.rs`、`woff2.rs`、`face.rs`）、`net/fonts.rs`（取字体、`font-display` 计时）、`css/stylesheet.rs`（`Rule::FontFace`）、`platform/mod.rs`（4 个新方法）；`qinglan-win` 的 `sys/dwrite.rs`（`IDWriteFactory5` 内存加载器）和 `platform.rs`；窗口 `qinglan-app`（`TIMER_FONT`、字体到达后合并进现有重排节流）；命令行 `render`/`record` 取字体、`wpt --win`。测试：`crates/qinglan-engine/tests/fonts.rs`（解码器）、`crates/qinglan-win/tests/webfonts.rs`（DirectWrite）、`crates/qinglan-cli/tests/hostile.rs`（3 个恶意字体）、WPT `css/css-fonts`。工具：`tools/make_font_vectors.py`、`tools/gen_brotli_tables.py`（只在做测试数据、生成表时用，不进产品）。

### `@font-face`（css-fonts-4 第 4 节）
- **从规则块的原文读，不用组件值**：`unicode-range: U+1E00-1EFF` 经过 CSS 分词变成数字和量纲（`1E00` 被当成指数），丢了十六进制。`fonts::face::parse_font_face` 自己按分号、逗号、引号、括号切文本。描述符：`font-family`（字符串或标识符序列，通用字体名无效）、`src`（`url()`、`local()`、`format()`、`tech()`）、`unicode-range`、`font-weight`/`font-stretch`/`font-style`（范围；`oblique` 带角度）、`font-display`；其余忽略（`size-adjust`、`ascent-override`、`font-variation-settings` 等没做）。没有 `font-family` 或没有可用 `src` 的规则丢掉。`url()` 按样式表地址取绝对地址。
- `format()` 认 woff2、woff、truetype、opentype 和它们的 `-variations`，别的（svg、embedded-opentype）这一条跳过；`tech()` 认 variations、features-opentype、color-COLRv0/v1、palettes。
- 同名规则都留着（保持声明顺序），`@media`/`@supports`/`@layer` 里的按条件算（走 `RuleSet::add_rules`），上限 256 条。

### 字体匹配（5.2）
- `rank_faces`：同一个 family 的所有字面按"拉伸（想要 100%）、样式、粗细"排成一个次序，同分时后声明的在前。粗细按 5.2 的三条规则（< 400、> 500、400 到 500 先升到 500 再降再升），样式按 normal/italic/oblique 的优先次序，拉伸先包含再窄再宽。排出的次序对每个字符都成立，所以平台只要"次序中第一个 `unicode-range` 含这个字符、字体里也有这个字符的"，结果和 5.2 对"含这个字符的字面"做匹配一样（测试 `font-weight-search-direction` 的网页字体那一半对）。
- **没有 `font-stretch` 属性**：样式引擎里没有（`FontQuery` 没有拉伸），所以只会要 `normal`；描述符读了、排了，但选不到别的宽度。WPT `font-stretch-02…15` 因此失败，写在 `notes.txt`（`limit (font-stretch)`），留给以后的一步。
- **带 `unicode-range` 的字面会让"第一个字体"变化**（5.1 "first available font"）：`resolve_font` 返回家族表里第一个有空格（U+0020）的字体，它给 `ex`、`ch` 和 strut；不含空格的子集字面跳过。`first-available-font-001…007` 全过。
- **有同名网页字体的 family 不再用安装的同名字体**（4.1）；一个字面没加载好、失败、或还在 `swap` 以外时，家族表里没有它，字符落到下一个 family。

### 取字体（`net/fonts.rs`）
- 样式算完就开始：`PageResources::start_fonts` 把所有字面告诉平台（`set_web_faces`，全部"不可用"），然后只取页面用得到的：对每个不同的（family 列表、粗细、斜体）——来自所有计算样式去重——按 `rank_faces` 的次序取，跳过 `unicode-range` 和页面文字用到的 Unicode 块（每块 256 个码位，扫一遍文本节点）都不相交的字面，跳过范围和已选字面完全相同的。没有 `@font-face` 的页面一点工作都没有（`set_web_faces(&[])` 在平台上是空操作）。
- 取字体是子资源，`ResourceKind::Font`：混合内容（https 页的 http 字体）拒绝、`file:` 页可以加载 `file:` 字体、`data:` 可以，规则和图片完全一样（`policy::plan` 没有为字体改一行）；优先级和样式表同档；响应上限 8 MB。**不做 CORS**：字体是跨源取的、不检查 `Access-Control-Allow-Origin`——这个浏览器没有脚本，没有跨源读取的问题，但和浏览器不一样（浏览器会拒绝没有 CORS 头的跨源字体）。这一条是有意的偏离，记在这里。
- `src` 按顺序试：`local(名字)` 用安装的字体族（`FindFamilyName`，不认全名和 PostScript 名）；`url()` 请求，失败（网络、HTTP 错误、解码失败、DirectWrite 不收、超过合计上限）就试下一条；都不行字面失败。
- **`font-display`**（4.5）：`auto`/`block` 3 秒块期，之后一直换；`swap` 没有块期；`fallback` 100 ms 块期、3 秒换期；`optional` 100 ms、没有换期（块期之后才到的字体不用）。块期里字面是"隐藏"：这个字面 `unicode-range` 内的字符用下一个字体的字形和宽度排版但不画（`HIDDEN` 位，见下）；块期结束（`TIMER_FONT`）变成"不可用"，字体到了再变成可用；每次变化都请求一次重排，合并进现有的重排节流（`relayout.request()` + `plan_relayout()`，滚动时推迟）。注意：这样的重排也计入 `image_relayouts`。
- **`ex`/`ch` 在窗口里用回退字体量**：样式在取字体之前算，窗口不为字体重算样式；命令行（`render`、WPT）在字体到齐后重算一次样式。记为限制。没做合成粗体/斜体（网页字体缺粗体时不加粗；系统字体 DirectWrite 照旧合成）、`size-adjust` 和各种 override。

### 解码器（纯 Rust，`forbid(unsafe_code)`，clippy 禁下标和 panic；输入全不可信）
- **inflate**（RFC 1951）+ zlib 外壳（RFC 1950，核对 Adler-32）：一个按 8 位查表、更长的码一位一位走的前缀码解码器（`huff.rs`，Brotli 共用）；输出上限由调用方给（WOFF 里是该表声明的 `origLength`），超了立即 `TooLarge`；每个块至少吃输入，所以工作量和输入加（有上限的）输出成正比。
- **Brotli**（RFC 7932）：完整：窗口位、元数据块、非压缩块、块类型切换、上下文映射（含逆移到前）、简单和复杂前缀码、距离短码和 NPOSTFIX/NDIRECT、静态字典和 121 种变换（表由 `tools/gen_brotli_tables.py` 从 google/brotli 的 `transform.c`/`context.c` 生成，UTF-8 上下文表 512 字节）。元块声明的长度在读它之前就和剩余上限比；一个命令至少吃输入或产出 2 字节，字典词变换成空的算错（防零进度循环）。**静态字典**（122,784 字节）以 deflate 压缩的形式放在 exe 里：**58,898 字节**（zlib 9 级的原始 deflate；`brotli_dict.z`），第一次有流引用字典时 inflate 出来放进 `OnceLock`（进程里留 122 KB 堆）。
- **WOFF 1.0**：头、目录、每张表 zlib 或原样；声明的 `totalSfntSize` 和各表 `origLength` 之和先和 8 MB 比；表按标签排序重写成 sfnt（`sfnt.rs`：4 字节对齐、重算目录校验和和 `head.checkSumAdjustment`）。
- **WOFF2**：表目录（UIntBase128、63 个已知标签号、变换版本）、一个 Brotli 流、`glyf`/`loca` 变换的还原（5.1：各个流、三元组坐标、复合字形、bbox 位图、`overlapSimpleBitmap`）、`hmtx` 变换（5.4）。**字体集（`ttcf`）拒绝**（`Unsupported`），不便宜。还原出的字形用标准 glyf 编码重写（带重复标志压缩）。声明的各表长度之和先和上限比，Brotli 的输出上限就是这个和，实际长度必须相等。
- **恶意输入**：`tests/fonts.rs`：Brotli 官方测试向量（`google/brotli` 的 `tests/testdata`，MIT，`tests/fixtures/brotli/LICENSE`）14 对加 `empty.compressed.00…18`、`x.compressed.00…03` 共 37 个，全部和期望输出一致；`monkey.compressed` 的每个前缀、每一位翻转、`quickfox_repeated.compressed` 每一位翻转都不 panic；WOFF2/WOFF 的前缀和改字节；声明 1 GB 的 WOFF2/WOFF（整体和单表）在分配前 `TooLarge`；Brotli 炸弹（600 个 1 MB 元块，输入 750 KB）在上限处停。`hostile.rs` 三个整页用例：WOFF2 声称 1 GB 的表、Brotli 炸弹、zlib 炸弹（声明 4 MB、流 100 MB）。向量字体来自 WPT 的 `fonts/`（OFL/BSD，`tests/fixtures/fonts/NOTICE-fonts.txt`）：WOFF 3 个（和期望的 sfnt 逐字节相同）、WOFF2 2 个（Noto Sans N'Ko、Mongolian：除 `glyf`/`loca`/`head` 外各表逐字节相同，字形头相同，字形外形由 `qinglan-win/tests/webfonts.rs` 用 DirectWrite 画出来逐像素比，相同）。期望输出由 fontTools（`tools/make_font_vectors.py`，只在做数据时用，不进仓库的构建）解出。

### DirectWrite（`sys/dwrite.rs`、`platform.rs`）
- `IDWriteFactory5::CreateInMemoryFontFileLoader`（Windows 10 1809 起）。槽号按 `dwrite.h`/`dwrite_3.h` 逐个数，用脚本核对：`IDWriteFactory` 9 CreateFontFace、13 RegisterFontFileLoader、14 UnregisterFontFileLoader；`IDWriteFactory5` 44 CreateInMemoryFontFileLoader（IID 958DB99A-BE2A-4F09-AF7D-65189803D1D3）；`IDWriteInMemoryFontFileLoader` 4 CreateInMemoryFontFileReference；`IDWriteFontFile` 5 Analyze。`WebFontLoader` 持有加载器（注册着）和字体字节（加载器可能直接用它们，所以放到字面全丢掉之后才放；也可能多一份拷贝）。DirectWrite 不收的字体（`Analyze` 不支持）→ `load_web_face` 返回 false，字面保持不可用。
- **字体号**：`PlacedGlyph::font` 是 `u16`，所以网页字体号放在 0x4000 起（一页最多 0x3000 个），0x8000 位是"隐藏"；系统字体号（≤ 512）在下面。网页字体在自己的表里（`web_fonts`），换页时清掉（字面全丢，字体面释放，加载器注销），字体号从 0 重新用，所以同时清掉按字体号做键的缓存（字形遮罩、单字、词）。字面状态一变，所有按查询算好的东西（家族表、选字体的缓存、整形缓存）作废。
- 一个字面的 `unicode-range` 放在字体条目里（`covers` 检查它），所以家族表里的字面像普通字体一样参加"第一个有这个字符的"。隐藏字面在 `items` 里占位；字符落在隐藏字面的范围内就带上 `HIDDEN` 位，宽度和字形来自后面的字体，`glyph_mask` 返回 `None`。
- 测试：WOFF2 解出的字体和同一字体的 TrueType 画出的字形逐像素相同（19 个字符）；坏字体被拒；`unicode-range` 之外落到下一个字体；隐藏时占位相同、不画、度量有；换成"不可用"后和没有网页字体时的整形相同。

### 数据（release；插电 `BatteryStatus = 2`；机器上有一个 Emulator.exe，CPU 负载约 25%，没有 cargo/rustc/qpdf 在跑）
- **exe**：2,479,616 字节，比 2d-1 之前的 2,268,672 多 **210,944**，**超出 2d-1 的 +150 KB（2,422,272）57,344 字节**。没有压回去：静态字典 58,898（固定），其余约 153 KB 是解码器、`@font-face` 解析、取字体、DirectWrite 加载和窗口代码；把解码器的位读取、前缀码解码、`Rd::take` 标成不内联省了 17,920。继续压要么放弃 WOFF2（字典），要么把解码器放到 `opt-level = "z"` 的单独 crate，这一步没做，留给第 2 层最后统一压。
- **内存**（`qinglan.exe --bench`，zhwiki-rec 1920×1080，5 次新进程，滚到底）：46.05、45.99、46.01、47.41、45.56 MB，最大 47.41 MB（≤ 50）。
- **首屏**（同上，5 次，毫秒，排序）：zhwiki-rec 127.4 / 140.1 / 140.9 / 144.3 / 187.6（中位数 140.9）；gh-ripgrep 131.1 / 178.6 / 189.6 / 211.6 / 213.4（中位数 189.6，抖动大，两个里最慢的两次超过 200）。两页都没有 `@font-face`，所以网页字体对它们只多了 `set_web_faces(&[])`（空操作）和 `resolve_font` 里一次缓存的主字体。
- **WPT** `css/css-fonts`（`--win`，51 个）：**32 / 51（62.7%）**，bug 类 3（`separators`、`font-default-01`、`font-face-unicode-range-2`，各有 `notes.txt` 一行说明没修的原因），其余 16 个是已知限制（`font-stretch` 属性 12、`rex`/`ric` 单位 2、系统字体粗细匹配 1、默认字体 1，都在 `notes.txt`）。`wpt --check` 2435 / 2435（原 2403 + 新的 32，`passing-win.txt`）。怎么挑的：`tests/wpt/README.md`。
- **真实网页**（`qinglan-cli record` + `render --replay`，1280×2400）：gov-uk 声明 4 个字面、加载 2 个，页面用的 GDS Transport 字体出来了（和 Edge 的字形一致）；mdn-div 声明 13 个、加载 6 个；jvns 声明 20 个（KaTeX 数学字体）、没取（页面没用）。旧的录制里没有字体文件（当时不取），所以重新录了这 3 页的字体。

### 没做 / 留给后面
- `font-stretch` 属性；`size-adjust`/`ascent-override` 等描述符；`font-variation-settings`；合成粗体斜体；窗口里字体到达后为 `ex`/`ch` 重算样式；字体集（`ttcf`）的 WOFF2；彩色字体的 `tech()` 之外的（2d-2 做彩色 emoji）。
- 评审（2d 之后统一做）要看的：`woff2.rs` 的 `reconstruct_glyf`（三元组、复合字形、bbox 位图）、`brotli.rs` 的 `meta_block`（块切换、上下文映射、字典引用）、`fonts/face.rs` 的文本解析（括号和引号嵌套）。

## 第 2 层 · 2d-2a 彩色 emoji 和从右往左（2026-10-20）

代码：引擎 `layout/bidi.rs`（新）、`layout/inline.rs`（双向分析、按层断开整形、行内重排）、`layout/block.rs`（10.3.3、外侧列表标记）、`layout/table.rs`（折叠边框镜像）、`layout/boxtree.rs`、`css/`（`unicode-bidi`、`DeclProp::Logical`、`dir=auto`、UA 样式）、`platform/mod.rs`（`bidi_levels`、`shape_rtl`、`GlyphMask::bgra`）、`platform/test.rs`（测试平台的小型 UAX #9 和彩色字形）、`paint/raster.rs`；`qinglan-win` 的 `sys/dwrite.rs`（`AnalyzeBidi`、`isRightToLeft`、`TranslateColorGlyphRun`）和 `platform.rs`。测试：`crates/qinglan-engine/tests/bidi.rs`、`tests/pages/bidi.html`、`rtl-block.html`（手算，页首注释写了推算，`check:` 行可执行）、`layout/bidi.rs` 单元测试、`crates/qinglan-win/tests/emoji_bidi.rs`；WPT 三个目录。

### 彩色 emoji
- `IDWriteFactory2::TranslateColorGlyphRun`，槽号按 `dwrite_2.h` 数：26 GetSystemFontFallback、27 CreateFontFallbackBuilder、28 TranslateColorGlyphRun（枚举器 3 MoveNext、4 GetCurrentRun）。Segoe UI Emoji 是 COLR/CPAL。每层用 `CreateGlyphRunAnalysis`（和普通字形一样：三个条纹取平均当灰度）画出覆盖率，平台把各层按调色板颜色从下往上 source-over 叠成**一张预乘 BGRA**（`GlyphMask::bgra`，`alpha` 为空），引擎的 `blend_color` 直接叠到画面上，文字颜色不参与。缓存和普通遮罩同一个 LRU（`GLYPH_CACHE_BYTES`），每项按 `alpha + bgra` 的字节数记。
- 调色板号 0xFFFF（该层用文字颜色）画黑色：平台不知道文字颜色；Segoe UI Emoji 里没见到用它的。位图（CBDT/sbix）和 OpenType-SVG 彩色字体**没做**，这些字体的字形按普通轮廓画（单色）。彩色字形不画文字阴影（阴影用 `alpha`，是空的）。
- 每个字体先问有没有彩色层：连续 64 个字形都没有彩色层就不再问（普通字体只多这 64 次调用），出现过一次彩色字形就一直问（Segoe UI Emoji 里也有数字等没有彩色层的字形）。
- **序列**：ZWJ 序列、肤色、VS16、keycap 整串交给一次 `GetGlyphs`，结果是**一个簇**（所有字形的 `cluster_start..cluster_end` 是整串），布局里是一个单位，不会被换行拆开。Windows 11（build 26300）的 Segoe UI Emoji 把 👨‍👩‍👧 排成同一个簇里的 3 个字形（字体自己叠），不是 1 个字形；画出来就是一个家庭图标，和 Edge 一致（测试 `emoji_sequences_are_one_glyph` 查的是“一个簇、全是 Segoe UI Emoji、有彩色”）。国旗：Windows 的 Segoe UI Emoji 没有国旗字形，两个区域指示符画成字母（Edge 也一样），不处理。
- **字体选择**：字符后面紧跟 U+FE0F 时先试 Segoe UI Emoji（❤️、1️⃣ 否则会落到 Segoe UI Symbol 的单色字形或 Segoe UI 的 "1"）。后面的 FE0F、20E3、ZWJ 本来就跟前一个字符的字体。
- **运行切断（1b 审查 R5）**：平台的 `run_cut` 和引擎的 `shape_cut` 的“后面不能断”表加了肤色修饰符 U+1F3FB–1F3FF 和标签字符 U+E0020–E007F（旗帜序列）；以前只有 ZWJ、VS、组合记号。区域指示符对没加（只有超过 8192 单元的单词正好卡在一对中间才会断）。

### 双向文字（UAX #9）
- **分工**：平台只给层级（`Platform::bidi_levels(text, rtl)`，每个字符一个层级，Windows 用 `IDWriteTextAnalyzer::AnalyzeBidi`）；引擎负责把 CSS 变成字符、重排每一行、对齐。`unicode-bidi` 变成 css-writing-modes-3 2.2 的控制字符（embed → LRE/RLE…PDF，isolate → LRI/RLI…PDI，bidi-override → LRO/RLO…PDF，isolate-override 两者，plaintext → FSI…PDI），行内盒的两端各一个不占位置的 `Ctl` 项；原子行内是 U+FFFC；强制换行是 U+2028（空白类，**所以 `<br>` 不开新的双向段落**，见“没做”）。块自己的 `bidi-override` 包住整个内容；块的 `plaintext` 用第一个强字符定段落方向（`bidi::first_strong`，跳过隔离里面的）。
- **DirectWrite 实测**（`sys/dwrite.rs` 的 `probe` 测试打印）：`AnalyzeBidi` 认 LRE/RLE/LRO/RLO/PDF 和隔离控制字符，隔离里面的层级对；隔离里的 PDI 本身带里面的层级（标准说是外面的），不影响显示，因为控制字符不出现在单位里。`GetGlyphs(isRightToLeft=true)` 的字形和簇映射仍是**逻辑顺序**（簇映射单调增），括号等镜像字符它自己换成镜像字形（`(` 的字形号等于 LTR 的 `)`），阿拉伯字母的连写字形和 LTR 不同（所以 RTL 一定要用 RTL 标志整形，`shape_rtl`）。右到左的整形不走词缓存和单字缓存，整段 script run 一次整形，缓存键里有方向位。
- **快速路径**：只有这几种段落才分析：块的 `direction: rtl`、`unicode-bidi: plaintext/bidi-override/isolate-override`、里面有会让从左到右文字重排的盒（rtl 的 embed/isolate、任何 override、plaintext）、文字里有右到左字符或显式控制字符（`textclass::is_bidi_relevant`：U+0590–08FF、FB1D–FDFF、FE70–FEFF、10800 起、200E/F、202A–E、2066–9；先用“有没有 ≥ 0xD6 的字节”过滤，拉丁和中日韩文字不用逐字符查）。其余段落没有任何额外工作；测试 `a_page_without_right_to_left_text_is_not_analysed` 数平台被问了几次（0 次）。
- **整形和单位**：整形前先有层级，文字按层级切成片段，奇数层用 `shape_rtl`；单位仍是逻辑顺序（换行不变，仍是逻辑的），每个单位记层级。右到左片段的字形数组里，簇**倒着存**（簇内的字形顺序平台已经放成从左到右的绘制顺序），这样一行里按视觉顺序排的单位，字形范围是连续的，一段连续的单位就是一个文字片段（`Group`；不连续就分成两个片段）。片段的 `start..end` 是它覆盖的字节范围（最小到最大）。
- **一行**：先按逻辑顺序量宽度和断行，再做 L1（行尾的空白、制表符和它前面的空白放回段落层级）和 L2（`bidi::reorder`），按视觉顺序排各组的 x。制表位按逻辑位置算（和浏览器不同，很少见）。两端对齐按逻辑顺序加宽，不受重排影响。
- **盒的边**：行内盒的起始边在 ltr 盒是左边、rtl 盒是右边（CSS2 9.4.2），`edge_widths` 按盒的 `direction` 取；边这个单位取它所在的嵌入层级（引擎自己按控制字符和文字里的 202A–E、2066–9 记栈）。被重排切开的盒，每一段视觉上连续的部分各画一个盒片段（`Ent::done`），起始边只在含它的那一段、结束边只在含它的那一段画边框；片段里 `first`/`last` 是物理的左/右。
- **对齐**（css-text-3 7.1）：`start`/`end`/`match-parent` 看段落方向（`match-parent` 按父块的方向把父块的 start/end 换成 left/right）；rtl 时 `text-indent` 在右边；行比盒宽时从起始边对齐，向另一边溢出（rtl 向左，偏移是负的）；悬挂的空格 rtl 时挂在左边。
- **逻辑属性**（css-logical-1）：解析时仍按 ltr 映射成物理属性，但这些声明记成 `DeclProp::Logical`，层叠时按这个元素自己的 `direction` 决定是不是换成对边（`PropId::mirrored`；`declared_rtl` 先扫一遍这个元素的 `direction`）。涉及 `*-inline-start/end`、`border-inline-start/end` 简写和四个 `border-start-start-radius` 等。UA 样式的列表缩进、`dd` 改成 `padding-inline-start`/`margin-inline-start`，作者写的 `padding-left: 0` 照旧盖得过（同一个物理属性，后者层叠顺序高）。`float`/`clear` 的 `inline-start`/`inline-end` 在样式里按**父元素**的方向变成 left/right。
- **`dir`、`<bdi>`、`<bdo>`**：UA 样式照 html-rendering 15.3.6：`[dir]:dir(ltr/rtl)`、`bdi:dir(…)`、`bdi, output, [dir=…] { unicode-bidi: isolate }`、`bdo { isolate-override }`、`pre[dir=auto] { plaintext }`；`:dir()` 认 `dir=auto` 和没有 dir 的 `<bdi>`（`direction_of`/`auto_rtl`，HTML 3.2.6.1：后代文字里第一个强字符，跳过 bdi、script、style、textarea 和自己有 dir 的元素）。块级元素的 `unicode-bidi: isolate`（UA 样式里那一长串）**没写**：块容器自己是段落，没有效果，写上每个 div/p/li 都要多算一份 `MiscStyle`（首屏变慢）。`plaintext` 块里的匿名块继承 `plaintext`。
- **块布局**：10.3.3 过约束时按**包含块**的方向丢掉右（ltr）或左（rtl）外边距（`solve_width` 加了参数）；外侧列表标记在 rtl 项目的右边（标记文字里的单词保持顺序、其余倒过来，“1. ”画成“ .1”）；折叠边框的表格 rtl 时列和边都镜像（边的上下左右按物理边取，`collapse_edges`）；flex 和 grid 在 2b 就认 `direction` 了。
- **命中测试和链接点击区域**：文字片段的位置本来就是视觉位置（`Fragment::x`），`interact` 按片段的盒找，没有改；一个链接被重排拆成几段时是几个片段，都能点。没有单独的测试（没有选择和光标，第 3 层）。

### 没做 / 记下的限制
- 被双向重排切开的行内盒，起始边和结束边应该贴在第一个/最后一个视觉片段的物理边上（浏览器的做法，`bidi-001`…、`bidi-box-model-*`、`bidi-005`…`011`）；我们按层级把边当单独的单位排，所以这些边和浏览器不同。WPT `css/CSS2/bidi-text` 因此有 34 个 bug 类失败（每个都有 `notes.txt` 一行）；要改要在 `lay_lines` 里把边从排序里拿出来，重排后再插到对应片段的两端。
- `plaintext` 的段落在强制换行（`<br>`、`pre` 里的换行）之后重新定方向：没做，整个块一个段落（WPT `bidi-plaintext-br-001`、`text-align-start-016`、`text-align-end-016`，都是 `limit`）。`<br>` 作为 U+2028 也影响换行两边的中性字符怎么归属。
- `text-align-last` 没有（WPT text-align 里 `-last-` 的 33 个）；`text-align: justify` 在带制表符、软连字符、双向控制字符的行里不完全按 7.3。
- 绝对定位盒的静态位置、滚动容器的起始位置、`<input>` 的方向、`text-align: start` 之外的 `direction` 相关默认值（表格单元格 `th` 居中等）没有按 rtl 改。竖排（2d-2b）以后在 `direction` 之上再加。
- 一个词里跨样式边界的连写（`ab<span>c</span>`，阿拉伯文里很常见）：Windows 平台是每个文字项单独整形，所以**样式边界处不连写**（和 Blink 不同，Blink 整个词一起整形）。这在 arwiki 的链接和粗体边界上没有看到问题（阿拉伯文维基正文里链接多在词的边界），但词中间换样式时会断开连写，记为已知限制；要做需要把同一单词的相邻文字项合并整形。
- `GlyphMask` 的颜色层固定 BGRA 预乘；高 DPI 下按放大后的字号取，所以不糊。

### 数据（release；插电 `BatteryStatus = 2`；Emulator.exe 在跑，CPU 约 9%，没有 cargo/rustc/qpdf）
- **exe**：2,517,504 字节，比 2d-1 之后的 2,479,616 多 37,888（本步预算 +40 KB = 2,520,576，余 3,072；没有单独压）。`wpt --check` 2635 / 2635（原 2435 + 本步新增，`list-marker-symbol-bidi` 和两个 grid 竖排测试退出通过名单，各有 notes 说明）；`cargo clippy --workspace --all-targets` 无警告；`cargo test --workspace` 439 个全过（`hostile` 里的网格计时用例在一次整套运行里因系统时钟跳变失败，单独重跑 31.9 秒通过）。
- **内存**（`qinglan.exe --bench`，zhwiki-rec，1920×1080，5 次新进程，滚到底，Private Bytes）：46,903,296 / 48,283,648 / 48,119,808 / 46,403,584 / 46,829,568，最大 48.28 MB（46.0 MiB），≤ 50 MB。
- **首屏**（同上，毫秒）：zhwiki-rec 129.1 / 136.1 / 135.3 / 121.5 / 127.0（中位数 129.1）；gh-ripgrep 138.2 / 126.0 / 130.7 / 115.7 / 132.5（中位数 130.7），都 ≤ 200。
- **WPT**（测试平台）：`css/css-writing-modes`（bidi 相关 98 个）83 / 98（84.7%），bug 类 0，15 个 `limit`：12 个是测试按 ezra 窄网页字体写的、盒宽 10em，测试平台 1em 的字形让换行和参考页不同（把盒放宽到 60em 的临时副本里这 12 个都通过，副本里整体 95 / 98）；3 个是 `plaintext` 要在 `<br>` 之后重新定方向（`bidi-plaintext-br-001`、`block-plaintext-004`、`block-plaintext-006`，放宽后仍不过）；`css/CSS2/bidi-text` 69 / 103（67.0%），bug 类 34（上面第一条）；`css/css-text/text-align` 51 / 85（60.0%），bug 类 0（`text-align-last`、justify 细节、plaintext 各有 `limit` 说明）。`--win` 对这些文字密集的页面不适合（字形抗锯齿位置不同，几乎全差几个像素），没有加进 `passing-win.txt`。
- **真实网页**：阿拉伯文维基“القاهرة”和英文维基 Emoji 条目，和 Edge 对比见 `tests/corpus/RESULTS.md` 最后一节。

## 第 2 层 · 2d-2b 竖排和注音（2026-10-21）

代码：引擎 `layout/vertical.rs`（新：框架、逻辑视图、逻辑→物理换算）、`layout/boxtree.rs`（框架标记、`ruby_pairs`、body 的书写模式传给根）、`layout/block.rs`（`layout_frame` 的入口、`Flow::inner`）、`layout/inline.rs`（字的方向、纵中横、注音的对齐）、`layout/position.rs`（rtl 的静态位置）、`css/`（`writing-mode`、`text-orientation`、`text-combine-upright`、`ruby-position`、`display: ruby-base`、UA 样式）、`paint/`（侧排字形旋转、直立字形的位置）、`textclass.rs`（`is_vertical_upright`）、`platform/mod.rs`（`shape_upright`）；`qinglan-win` 的 `sys/dwrite.rs`（`vert` 特性）和 `platform.rs`。测试：`tests/pages/vertical.html`、`ruby.html`（手算，页首写了推算，`check:` 行可执行）、`crates/qinglan-win/tests/vertical.rs`、WPT 两个目录。

### 先修 2d-2a 的回归：`bad_utf8`
- 现象：release 版 `hostile` 的 `bad_utf8`（4 MB 随机高位字节）从 `9c490b2` 的 105 MB / 1.96 s 涨到 `ced4dfd` 的 154 MB / 2.6 s，限额 120 MB。2d-2a 当时只跑了 debug 版，看不到。
- 原因：随机字节里有约 3% 是希伯来文、阿拉伯文（0xD6–0xDF 开头的两字节序列），所以这一页**要**做双向分析（不是快速路径）；分析时 `WinPlatform::bidi_levels` 给每个字符建了一个 `Vec<usize>`（容量 = 文字的**字节数**，3–4 MB 的文字就是 24–32 MB），旁边还有一个同样容量的 `Vec<u16>`。改成：UTF-16 缓冲区按实际单元数分配，层级直接逐字符映射，不要索引数组；`bidi::analyze` 的字符串也一次分够。结果：峰值 154 → 109 MB（限额 120），耗时 2.6–2.75 s（比 105 MB 那时的 1.96 s 慢是因为这一页确实要做双向分析、按层级把文字切成很多小片段整形；限额 5 s 内，没有再优化）。release 版 29 个恶意网页用例全过。

### 竖排（css-writing-modes-3）：怎么做
- **框架（frame）**：块的书写模式和包含块里内容的书写模式不同（横排页里的竖排块、竖排文字里的表格或图片），这个盒子是**一个框架的根**（正交流，7.3；同时是格式化上下文的根）。框架里面完全用现有的横排代码、按**逻辑**方向排（行内轴当 x，块轴当 y），排完后 `turn_frame` 一次把盒子、行、片段、装饰线、列表标记的位置换成物理的：`vertical-rl` 块轴从右往左（`x' = H - y - h`，H 是框架的块尺寸），`vertical-lr` 从左往右，行内轴都是从上往下，宽高对调。嵌套的框架已经是最终的物理位置，只换它自己的矩形。这样 flex、grid、表格、浮动、绝对定位、外边距合并在竖排里都不用另写一份，缺点是它们按**逻辑**排好再转（见限制）。
- **样式的逻辑视图**：框架里的盒子的 `style` 是 `logical_view`（`vertical.rs`）：宽和高对调，外边距、内边距、边框宽度/样式/颜色、`top/right/bottom/left`、`overflow-x/y` 按边映射换位（rl：逻辑上下左右 = 物理右下左上；lr：左下右上）；原来的物理样式放进 `Rare::phys`，绘制时用 `LayoutBox::pstyle()`（背景、边框、轮廓、圆角、阴影都按物理边画）。只有页面里有竖排的元素才做这个遍历（`Builder::vertical`），横排页面不付任何代价。框架根自己的 `style` 保持物理的（`froot`、`pf` 记着它所在的框架），它的外边距/尺寸在 `layout_frame` 里按物理数组和两边框架的边映射算。
- **正交流的尺寸**（7.3）：竖排盒的行内尺寸是物理高度：写了 `height` 就用它，`auto` 时是 fit-content，可用空间 = 包含块的高度（定义了才用），否则是视口高度（再减去自己的外边距、内边距、边框）；块尺寸（物理宽度）是内容决定的，`width: auto` 不再填满。`vertical-rl` 的根元素（或 `body` 的值传给根，见下）的行内尺寸填满视口高度，宽度是内容，放在视口右边（`x = 视口宽 − 宽`），所以第一列在右边；没有横向滚动，超出视口左边的列看不到（记下）。
- **body 的书写模式传给根**（8）：根是 `html` 且有 `body` 时，根的书写模式取 `body` 的值（`wm-propagation-body-*`）。
- **字的方向**（5.1）：`text-orientation: mixed` 里 CJK、假名、谚文、注音符号、全角和 CJK 标点符号、emoji 直立（`textclass::is_vertical_upright`，UAX #50 的 U/Tu 的常用部分），其余（拉丁、数字、`…`、`—`）转 90°；`upright` 全直立；`sideways` 和 `sideways-rl/lr` 全转。文字按方向切成片段整形。**直立片段**：DirectWrite 加 `vert` 特性（竖排字形：「」、、。在右上角、ー转向；`GetGlyphs` 的特性数组），整段整形、不走词缓存；每个字形沿行占 1em（`advance = 字号`，原来的宽度放在 `offset_x`，绘制时用它居中），整个片段是 `FragKind::Text { v: 2 }`，绘制时每个字形放进自己的格子，在行的中线上（基线 = 格子中点 + (上升 − 下降)/2）。**侧排片段**（`v: 1`）：照横排整形，绘制时字形遮罩转 90°（`paint/fx.rs::turn_mask`：遮罩的像素按 (u, v) → (−v, u) 重排，不重采样；子像素位置丢掉，沿行用四舍五入）；基线离片段左边的距离换算成物理。`vertical-lr` 里字形顶朝右（块结束侧），所以逻辑视图里字体的上升和下降对调（`font_for`）。`sideways-rl` 是 `vertical-rl` 加 `text-orientation: sideways`；**`sideways-lr`（字形逆时针转、行内方向从下往上）只解析，按 `vertical-lr` 加侧排画**（记下）。
- **纵中横**（9.1）：`text-combine-upright: all` 的文字项不超过 4 个字符时是一个 1em 的格子（一个单位）：宽度超过 1em 就按比例缩小字号重新整形（`FragKind::Text::fs` 记缩小后的字号），横排画在格子中间；`digits` 只认 2 个 ASCII 数字。超过 4 个字符不合并。每个文字项各自合并（嵌套的行内元素各合各的，规范说整个元素合成一个）。
- **`direction: rtl` 在竖排里**：框架里照横排的 rtl 排（行内起点在逻辑右边 = 物理下边），双向分析照常（侧排的拉丁/阿拉伯文在从下往上的行里顺序正确）。绝对定位盒的静态位置：`left` 和 `right` 都是 `auto` 且所在块是 rtl 时，静态位置是右边缘（CSS2 10.3.7；竖排里就是下边缘）；flex/grid 的子项有自己的静态位置算法，不改。
- **绘制**：直立和纵中横不用改光栅器（字形位置由偏移定）；侧排用 `TextRun::rot`，笔尖从 `baseline`（行起点）往下走，`x` 是基线；文字阴影在竖排里不画；装饰线随框架转成竖线（线在逻辑的下面 = 竖排的左边或右边，浏览器放在右边，没有照做）。

### 注音（css-ruby-1 基本部分）
- `ruby`、`rb`（`display: ruby-base`）、`rt`（`display: ruby-text`、`font-size: 50%`、`line-height: normal`）、`rp`（`display: none`，所以不显示）。箱子树里（`ruby_pairs`）把每一段底字和它后面的 `rt` 做成**一个匿名 `inline-block`**：里面两个块，一个是注音（`rt` 自己的盒），一个是底字（匿名块），按 `ruby-position` 排顺序：`over`（默认）注音在逻辑上方（横排 = 上方；`vertical-rl` 的逻辑上方 = 物理右边），`under` 在下方；`vertical-lr` 的逻辑上方是左边，而注音要在右边，所以 `over` 放在逻辑下方（css-writing-modes-3 6.1）。两个块的 `text-align` 是内部值 `-qinglan-ruby-space`（`ruby-align: space-around`：多余的空间平均分给每个字，两端各半份；单个字就居中），宽度是两者里宽的那个（`inline-block` 的 shrink-to-fit），所以窄的一方居中或疏排。这一对按**底字所在块的基线**对齐（`Rare::ruby`），注音撑高行盒只在超出行距的部分（对原子行内来说就是它的上升）。同一个底字后面的第二个 `rt`、没有底字的 `rt`、`rtc`、`ruby-merge`、`ruby-overhang`、注音溢出到相邻字上、一对注音里的断行：没做。
- 限制：一对注音是原子行内，行不能在这一对的里面断开（跨多个字的底字不断）；`display: ruby` 的元素本身仍是普通行内盒。

### 没做 / 记下的限制
- flex、grid、表格在竖排里按**横排**排（它们是 `horizontal_only` 的框架：自己的内容不转，只有它们自己的矩形跟着外面转），它们的子项如果继承了竖排，则是正交的框架，但 flex/grid 算法不按框架的块轴算大小。因此 WPT 里原来靠“竖排被忽略”碰巧通过的 31 个 flex/grid 竖排测试退出通过名单（`notes.txt` 每条有说明，类别 `bug (flex/grid with vertical writing)`）。
- 正交盒的固有宽度（给 shrink-to-fit 的外层用，如浮动、表格单元格、`inline-block` 里的竖排块）：算 0（`content_widths` 对框架根从外面问时返回 0）；框架根做原子行内时没有基线（放在底外边缘）。
- 竖排里的百分比高度（按包含块的**物理**高度）、`vertical-align` 的上下方向（不随 lr 翻转）、列表标记（外侧标记仍按横排画）、`text-indent`、`text-align: start/end` 在 `direction: rtl` 里照横排规则；`position: absolute` 在竖排框架里的各种方向组合（WPT `abs-pos-non-replaced-vlr/vrl` 共约 115 个、`sizing-orthog-*`、`available-size-*`、`clip-rect-*`：没有逐个诊断，分家族写在 `notes.txt`；`sizing-orthog-vrl-in-htb-001` 的参考页用竖排根 + 绝对定位，我们的根框架排出来的块尺寸和内边距不对）。
- 没有 `text-orientation` 的 `sideways` 以外的字形旋转角度、没有 `text-emphasis`、没有 `font-variant-east-asian`、竖排里没有 `letter-spacing` 的特殊处理。

### 数据（release；插电 `BatteryStatus = 2`）
- **exe**：2,567,168 字节，比 2d-2a 之后的 2,517,504 多 49,664（本步预算 +60 KB = 61,440 内）。
- **内存**（`qinglan.exe --bench`，zhwiki-rec，1920×1080，5 次新进程，滚到底，Private Bytes）：48,418,816 / 46,473,216 / 46,551,040 / 46,194,688 / 45,338,624，最大 48.42 MB（≤ 50 MB）；gh-ripgrep 22.0–22.8 MB。
- **首屏：没有合格的数字。** 量的时候另一个会话在编译（9 个 `rustc` 同时跑，另有 `Emulator.exe`），zhwiki-rec 161–663 ms、gh-ripgrep 152–434 ms，都不能和 200 ms 的限额比。竖排和注音的代码在不用它们的页面上不执行：箱子树的框架遍历只在页面里有竖排元素时才做，`font_for` 多一个布尔判断，光栅的 `glyphs_of` 多一个分支；这是推断，不是测量，要在空闲机器上补量。
- **WPT**（测试平台）：`css/css-writing-modes` 439 / 843（52.1%）（2d-2a 的 98 个加新增的竖排 745 个）；bug 类 388，**没有修**，每个都有一行 `notes.txt`（按家族写的原因，上面“记下的限制”里没有诊断的那些；写明了哪些诊断过：`sizing-orthog-vrl-in-htb-001` 的参考页、`wm-propagation-body-032` 的版面）。`css/css-ruby` 23 / 118（19.5%），失败的 95 个：65 个是 `step 2d`（`rtc`、`ruby-align`、`ruby-overhang`……）、30 个是 `limit (ruby as inline-block pairs)`（每个一行）。`qinglan-cli wpt --check` 通过。自动分类器原来把含 `writing-mode`、`direction`、`<ruby` 的页面都归“后面的步骤”，这一步改成只认 `rtc`、`ruby-align`、`ruby-merge`、`ruby-overhang`。
- **真实网页**：青空文库（注音）和 vert-tategaki（竖排）对 Edge，见 `tests/corpus/RESULTS.md` 最后一节。

## 第 2 层 · 2c/2d 审查修复（2026-10-21，提交 A：`Fix 2c/2d review findings: decoders and amplification`）

起点 `b7a437a`。代码：`fonts/brotli.rs`、`fonts/woff2.rs`、`fonts/sfnt.rs`、`layout/boxtree.rs`、`qinglan-win/src/sys/dwrite.rs`、`qinglan-cli/src/bench.rs`。测试：`tests/fonts.rs`（+1）、`woff2.rs` 里的单元测试（+1）、`tests/hostile.rs`（+4）。

### R1 Brotli 的准备工作量（`brotli.rs`）
- 原因：上下文映射逆移到前时对每一项做 `mtf.remove(i); mtf.insert(0, v)`，每项移动整个 256 字节的表；一个约 82 字节的元块（256 个字面块类型，1 个 16,384 项的上下文映射，一整段零）要 0.7 ms，984 KB 的字体（12,000 个这样的元块）8.5 s。
- 改成 `mtf[..=i].rotate_right(1)`，代价是 `i`，不是表长。另外给每个流一个**准备工作量预算**（`Work`）：前缀码（字母表大小 + 256 的查找表）、上下文映射（每项 1 单位）、逆移到前（每项再加 `i`）都要花，预算是 1M 单位 + 输入每字节 16 单位；用完就是 `TooLarge`。真字体的上下文映射用 RLE 和逆移到前编得很小，每字节输入花不了几个单位；上面那个 82 字节的元块要 17,000 多单位，所以 12,000 个元块在几百个之后就停了（约 40–50 ms）。块切换不单独计：每次切换至少吃 2 位输入，代价是 O(1)。
- **发现：`qinglan-cli bench` 根本不取网页字体**，所以 `hostile.rs` 里原来三个字体用例（`woff2_1gb`、`brotli_bomb`、`zlib_bomb`）一直没有碰到解码器（页面 30–50 ms、8 MB）。`bench` 现在在首屏之后调 `load_fonts`（和窗口一样，首屏的数字不变；没有 `@font-face` 的页面什么也不做），并打印 `web-fonts: N declared, M loaded, K failed`；`hostile.rs` 的字体用例核对这一行，保证字体真的被取了、送进了解码器、被拒了。
- 新用例：`brotli_maps_3000`、`brotli_maps_12000`（984 KB）、`brotli_maps_32_faces`（一页 32 个不同文件的这种字体，共 32 MB）。修之前这个流在 release 里要 8.5 s（`mtf12000.woff2`，审查的重现）。

### R3 注音对（`boxtree.rs::ruby_pairs`）
- 每对注音原来要 3 个盒子加约 10 份新样式（`with_align` 复制一份 `ComputedStyle` 和 `TextStyle`，`anon_style` 两次，匿名 `inline-block` 的 `BoxStyle`），10 万个 `<ruby>` 的 `layout` 峰值 356 MB。现在：(a) 按（`ruby` 的样式，`rt` 的样式）两个指针缓存一套三个样式（`ruby_styles`，最多 1024 套，满了清空；缓存里留着两个原样式的 `Rc`，指针不会被别的样式占用）；(b) 每对（含被丢下的 `rt` 盒，共约 4 个盒子）计入生成盒子的额度（`generated` / `gen_cap`，和 `::before`/`::after` 同一个），额度用完，后面的底字和 `rt` 就是普通行内盒（`rt` 紧跟在底字后面，字号 50%），`gen_cut` 置位，页面“不完整”。
- 结果（10 万个 `<ruby>字<rt>注音</rt></ruby>`，40 万个文档节点，额度只剩 2.5 万个盒子）：`layout` 2.68 s / 356 MB → 0.91 s / 156 MB；`hostile.rs` 的整条流水线（`bench`）0.77 s / 185 MB（限额 200）。

### R5 WOFF2 的临时拷贝（`woff2.rs`、`sfnt.rs`）
- 重建出的 `glyf`、`loca` 原来在 `raw`（Brotli 的输出）和 `rebuilt` 都还在的时候又 `clone` 一份，没变换的表又 `to_vec`，再加上 `assemble` 写出的整个文件：最坏时 `raw` 8 + 重建 8 + 表 16 + 文件 8 MB。现在：重建的表 `mem::take` 移走不复制；复制没变换的表之前先核对“重建的 glyf+loca + 没变换的表”不超过 8 MB（`TooLarge`）；表都拿到手之后先放掉 `raw`，再 `assemble`；`assemble` 写完一张表就放掉一张；`glyf` 按变换后大小的 1.5 倍一次分够（`Vec` 的翻倍在 8 MB 上会要 16 MB）。最坏的峰值是 `raw`（≤ 8）+ 表（≤ 8）= 16 MB，然后是表（≤ 8）+ 文件（≤ 8）= 16 MB，也就是上限的 2 倍。**没有测量**（要造一个 8 MB 的 `glyf` 变换流，这一轮没做）。

### R6 两处
- `woff2.rs`：`encode_points` 里相邻两点的差（坐标本身在 i16 里，差可以到 ±65535）原来 `as i16` 截断，字形悄悄变形；现在装不下就是 `Corrupt("coordinate delta out of range")`（单元测试：差 60,000 拒绝，20,000 通过）。
- `dwrite.rs::rasterize_color`：各层像素合计上限 16 Mpx（`MAX_COLOR_LAYER_PIXELS`，每层 1 字节，量的时候另有 3 倍的临时纹理），合成的外框上限 1 Mpx（`MAX_COLOR_PIXELS`，累加器每像素 16 字节，原来 2048×2048 就是 64 MB）；超了返回 `None`，调用者按单色画这个字形。

## 第 2 层 · 2c/2d 审查修复（2026-10-21，提交 B：`Shadow masks: 9-patch cache`）

代码：`paint/fx.rs`。测试：`paint/fx.rs` 里 3 个单元测试，`tests/effects.rs` +1，`tests/hostile.rs` +3。

### R2 盒阴影遮罩改成 9 宫格（`ShadowGeometry`、`Patch`、`shadow_patch`）
- 原来：遮罩是整个窗口（盒子加两边的模糊半径）大小，超过 `MAX_KEPT_PX`（1 Mpx）的每一帧都重做（一张 1270×1000 的盒阴影每次约 50 ms，r1_shadow_only 300 个共 10–15 s），超过 `MAX_MASK_PX`（4 Mpx）整个不画（1260×3300 的包装层没有阴影）。
- 现在：离边和角够远的地方，模糊后的值沿一个轴不变——形状在 `e0 + r0 .. e1 - r1`（两边的角之间）沿这个轴是平的，一个像素只看到两边各 `pad`（`reach(σ)`，至少 1）以内，所以窗口里 `m0 = e0 + r0 + pad .. m1 = e1 - r1 - pad` 这一段的每一列（行）的值都一样。把这一段切掉、只留左边 `m0 + pad` 列和右边 `W - (m1 - pad)` 列，在这个缩小的形状上做一遍（形状 + 模糊），读的时候窗口的第 x 列去读：x < m0 读第 x 列，`m0 ≤ x < m1` 读第 m0 列，x ≥ m1 读第 `x - (m1 - m0) + 2·pad` 列；行同理。盒式模糊每一遍的整数取整在平移下不变，所以这样读出来和整个窗口做的**逐像素相同**（单元测试：方角精确相等，圆角差 ≤ 1，因为弧线的折线从不同的坐标算起；外阴影、内阴影、洞的边在窗口外的内阴影、窄盒子只切一个方向、σ = 0 都测了）。切掉的行列少于 `2·pad + 8` 就不切。
- 缓存的键是缩小后的形状（缩小后的宽高、四条边、圆角、σ、内外），不是盒子的大小：300 个同样阴影的盒子共用一张遮罩，任何大小的卡片都是同一张。遮罩大小只和圆角与模糊半径有关，所以 `MAX_MASK_PX` 以后只挡住模糊半径大到上千像素的阴影。
- 画的时候：逐行把“不可能有阴影”的段去掉（外阴影：盒子里圆角够不到的中间一段；遮罩那一行整段是 0 时中间被切掉的一段，内阴影常见），其余的才取遮罩、算覆盖、混合。
- 模糊本身：列方向原来一列一列地拷出来再拷回去（每个像素碰一条新的缓存线），改成每列一个滑动和、一行一行地走（`box_cols`），两个缓冲区来回；除法换成乘法加移位（`Mean`，对 ≤ 255×4097 的和精确，单元测试对比除法）。
- **和旧代码的画面对比**（旧 = 提交 A 的 `qinglan-cli`，新 = 本提交，`render` 1280×1000，滚动 0 和 700）：p3_shadows16x、q3_shadows30、r1_both、r2_blur_big、r2_blur_distinct、q5_shadow400、bigcard2（2 Mpx 窗口）、以及 gh-ripgrep、gh-rust、mdn-div、mdn-flex、rust-book-own **逐字节相同**（最大差 0）；差别只在旧版本来就不画的 1260×3300 包装层（现在有阴影）。hn-front、gov-uk、jekyllrb 在旧版自己跑两次也有 80–383 个像素不同（图片解码的时序），与本改动无关。
- 数字（release，`render` 单次进程，1280×1000；旧 → 新）：r1_shadow_only（300 个 1270×1000 的盒阴影）10.3 s → **0.22 s**；q5_shadow400（400 层嵌套带圆角阴影）3.64 s → **0.36 s**；p3_shadows16x 约 1 s（16 层 100 px 模糊 × 300 个，不变，主要是风格数多）；1260×3300 包装层：没阴影 → 有阴影，`--scroll-bench` 每帧中位数 1.6 ms；1260×1800 的卡片（旧版窗口 2.4 Mpx，每帧重做）每帧 **63–66 ms → 1.6–1.7 ms**；同样大小带内阴影（`inset 0 4px 24px`）每帧 2.3 ms。`hostile.rs` 新增三个：300 个满屏盒阴影 0.16 s，400 层嵌套 1.2 s，3300 高的包装层 0.3 s。
