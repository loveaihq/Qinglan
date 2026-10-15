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
- 其他请求头：`Accept`（按类型：页面、样式表 `text/css,*/*;q=0.1`、图片列出常见格式）、`User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) qinglan/0.1`（任务单；常量 `net::USER_AGENT`，由传输设置）、`Accept-Encoding: gzip, deflate`（WinHTTP 开了解压就自动加）。**不发 `Accept-Language`**（任务单没写；发了就会泄露系统语言，网站按网址的语言版本也能工作）。不发 Cookie、也不处理 `Set-Cookie`（WinHTTP 的 `DISABLE_FEATURE` 关了 Cookie）。

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
