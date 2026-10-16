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
