# 第 1 层：能看文章

做完后有两个程序：

- `qinglan.exe`：一个窗口，能输入网址、打开网页、点链接、前进后退、滚动。
- `qinglan-cli.exe`：开发和测试用，不发给用户。能打印 DOM 树和盒子树、把网页渲染成 PNG、录制和回放网页、测速度和内存。

不跑 JS。暂不支持的排版按本单"退路"处理。

分三步做，每步做完都能测，测完提交一次，做一次清单式审查：

- **1a 解析**：骨架、网址、编码、HTML、CSS、层叠。只有命令行。
- **1b 文字和排版**：平台接口、DirectWrite 文字、盒子树、排版。
- **1c 网络、绘制、窗口**：WinHTTP、WIC、绘制、窗口、录制回放、真实网页和恶意网页测试、性能。

---

## 1a 解析

### 1. 项目骨架

- Cargo 工作区：
  - `crates/qinglan-engine`：内核（库）。纯 Rust，`forbid(unsafe_code)`，不依赖 Windows。
  - `crates/qinglan-win`：Windows 平台层（库）。`unsafe` 只在 `src/sys/` 下。
  - `crates/qinglan`：生成 `qinglan.exe`（1c 才有内容），`forbid(unsafe_code)`。
  - `crates/qinglan-cli`：生成 `qinglan-cli.exe`，`forbid(unsafe_code)`。
- Rust 2024 edition，`rust-toolchain.toml` 锁定当前装的稳定版（1.99.0）。
- `.cargo/config.toml` 开 `+crt-static`，exe 只依赖系统 DLL。release 开 LTO、`codegen-units = 1`。不设 `panic = "abort"`：页面级别要能 `catch_unwind`（见 1c）。
- 内核的 clippy 规则：禁止 `unwrap_used`、`expect_used`、`panic`、`indexing_slicing`（测试代码除外）。
- `LICENSE-MIT`、`LICENSE-APACHE`、`.gitignore`（`target/`、`docs/spec/`、`tests/corpus/local/`、`tests/out/`）。
- `git init`，仓库内设 git 身份 `Vincent <loveaihqhq@gmail.com>`（和 qingpdf 一样，不用工作邮箱）。第一次提交只含骨架和文档。

### 2. 规范

下载到 `docs/spec/`，只放本地。只下要用的章节，不整站下载：

- WHATWG：HTML（解析 13.2、渲染 15、命名字符引用 `entities.json`）、URL、Encoding
- W3C/CSSWG：CSS 2.2（第 8–10、16 章）、css-syntax-3、selectors-4、css-cascade-5、css-values-4、css-variables-1、css-color-4、mediaqueries-4、css-display-3、css-text-3、css-inline-3、css-position-3、css-fonts-4

### 3. 网址（WHATWG URL）

- 解析、相对网址拼接、序列化，用 `special scheme` 规则；主机名的 IDNA 转换（中文域名）用 Punycode 自己写，不做完整的 UTS #46 映射，只做小写化和 Punycode（记进 decisions.md）。
- 测试：WHATWG 的 `urltestdata.json`（来自 web-platform-tests，BSD-3）放进 `tests/wpt-url/` 并带许可证；通过率报出来，失败的逐条列原因。

### 4. 编码（WHATWG Encoding）

- 判断顺序：BOM → HTTP 头的 `charset` → 前 1024 字节里的 `<meta>` 预扫描（HTML 13.2.3.2）→ 都没有时用 UTF-8；但如果没有任何声明、而前 4 KB 按 UTF-8 解有错，改用 GB18030（中文老网站常见，记进 decisions.md）。
- UTF-8、UTF-16LE/BE 在内核里自己解；其余编码通过平台接口交给 `MultiByteToWideChar`。标签按 Encoding 标准的对照表：`gb2312`/`gbk`/`gb18030` → 代码页 54936；`big5` → 950；`shift_jis` → 932；`euc-kr` → 949；`iso-8859-1`/`us-ascii` → 1252（标准规定）；其余有对应代码页的照对。Windows 没有的编码当 windows-1252 处理，并给出警告。
- 坏字节一律换成 U+FFFD，不报错。
- 1a 阶段的测试平台可以只实现 UTF-8/UTF-16；GBK 等在 1b 接上 `qinglan-win` 后测。

### 5. HTML 解析（HTML 13.2）

- 分词器：全部状态；命名字符引用用完整表（从 `entities.json` 生成，压缩存放）；数字引用按标准的替换规则。
- 建树：全部插入模式，包括 adoption agency（错误嵌套的格式标签）、foster parenting（表格里放错位置的内容）、隐含结束标签、`template`、外来内容（SVG/MathML 照常建树，第 1 层不显示）。脚本开关为"关"：`<noscript>` 里的内容当普通 HTML 解析。
- 打开的元素超过 512 层后，新元素挂到第 512 层上（Blink 的做法），不再加深。
- 节点总数上限 50 万；超了丢掉后面的内容，页面顶部给一条提示"页面太大，只显示了前面部分"。
- DOM 存在数组里（节点用 `u32` 下标），标签名和属性名驻留成编号，方便快速比较。
- 测试：html5lib-tests（MIT）放进 `tests/html5lib/` 并带许可证。

### 6. CSS

**样式来源**：内置的浏览器默认样式（照 HTML 15 改写，编进 exe）、`<link rel=stylesheet>`（看 `media` 属性）、`<style>`、`style=""`、`@import`（最多 4 层、每页最多 64 个样式表），以及 HTML 15.3 里的老式表现属性（`bgcolor`、`align`、`<font color size face>`、`<body text link>`、`width`/`height` 等；中文老网站大量使用）。

**解析**：按 css-syntax-3 分词和解析，错误恢复照标准（写错的声明丢掉，前一个有效声明生效，网站的兼容写法就靠这个）。

**@ 规则**：

- `@media`：媒体类型 screen/print/all；`width`/`height`（含 min/max 和 `(width >= 600px)` 范围写法）、`orientation`、`resolution`、`prefers-color-scheme: light`、`prefers-reduced-motion: reduce`、`hover: hover`、`pointer: fine`；`and`/`or`/`not`/逗号。
- `@import`、`@layer`（按 cascade-5 排优先级，Tailwind v4 这类网站全靠它）、`@supports`（按我们实际能解析的属性和值判断，含 `selector()`）。
- `@font-face`、`@keyframes`、`@container`、`@page`、`@namespace` 等：第 1 层忽略（`@container` 里的规则不生效）。

**选择器**：类型、`*`、类、id、属性（7 种运算符和 `i` 标志）、四种组合符；`:root`、`:first-child`、`:last-child`、`:only-child`、`:nth-child(An+B)`、`:nth-last-child`、`*-of-type` 系列、`:empty`、`:not()`、`:is()`、`:where()`、`:link`、`:any-link`、`:lang()`、`:checked`、`:disabled`、`:enabled`。

- `:visited` 永远不匹配（隐私）。`:hover`、`:active`、`:focus` 第 1 层永远不匹配。`:has()` 第 1 层不支持（按标准，整条规则作废，`:is()`/`:where()` 里的宽容列表除外）。
- 伪元素 `::before`、`::after`、`::marker`、`::first-line`、`::first-letter`、`::selection`、`::placeholder` 要能解析（规则有效），但第 1 层不显示。
- 性能：规则按 id/类/标签名分桶，从右往左匹配，祖先用布隆过滤器加速。

**层叠和值**：

- 来源和 `!important`、`@layer`、优先级、先后顺序；继承；`initial`/`inherit`/`unset`/`revert`/`revert-layer`。
- 自定义属性和 `var()`（含后备值）：要检测循环引用；单个值展开后最长 64 KB，超了当作计算时无效。
- 长度：`px em rem ex ch % pt pc in cm mm Q vw vh vmin vmax`（`dvh`/`svh`/`lvh` 当 `vh`）；`calc()`、`min()`、`max()`、`clamp()`，嵌套最多 32 层。
- 颜色：148 个颜色名、`#` 三/四/六/八位、`rgb()`/`rgba()`/`hsl()`/`hsla()`/`hwb()`（逗号和空格两种写法）、`lab()`/`lch()`/`oklab()`/`oklch()`（换算到 sRGB）、`currentColor`、`transparent`、常用系统颜色。`color-mix()` 等其他写法当无效。
- 计算样式按组（盒子、外边距、边框、字体、文字等）用 `Rc` 共享；兄弟元素样式完全相同时整个共享。

**第 1 层要算出的属性**（下面没列的，解析了也不用）：

`display`（`block`、`inline`、`inline-block`、`list-item`、`none`、`contents`、`flow-root`；退路见 1b）、`position`（`sticky` 当 `relative`）、`top/right/bottom/left`、`z-index`、`float`/`clear`（只解析）、`box-sizing`、`width/height/min-*/max-*`、`margin*`、`padding*`、`border*`（宽、样式、颜色；`border-radius` 只解析）、`color`、`background-color`（`background` 简写只取颜色）、`font`、`font-family/size/weight/style`、`line-height`、`letter-spacing`、`word-spacing`、`text-align`、`text-indent`、`text-decoration*`、`text-transform`、`white-space`（及 `white-space-collapse`、`text-wrap-mode`）、`word-break`、`overflow-wrap`/`word-wrap`、`vertical-align`、`list-style*`、`visibility`、`overflow`、`cursor`、`direction`（只解析）、`lang` 相关的继承。

### 7. 命令（1a）

| 命令 | 作用 |
|---|---|
| `qinglan-cli dom <文件>` | 打印 DOM 树，用 html5lib 测试的格式 |
| `qinglan-cli style <文件> [--width 1280]` | 打印每个元素的计算样式（只列和默认值不同的） |

### 1a 验收

1. `cargo test` 全过、`cargo clippy` 无警告，贴输出。
2. html5lib 分词测试 100% 通过；建树测试（脚本开关为"关"的用例）≥ 98% 通过；没过的逐条列进 decisions.md 并写明原因。
3. URL 测试通过率和失败原因。
4. CSS 的单元测试：层叠顺序（来源、`!important`、`@layer`、优先级）、`var()` 循环和后备值、`calc()`、`@media` 在 360/768/1280 三种宽度下、`@supports`、颜色换算、错误恢复。
5. 性能：一个 2 MB 的大网页（中文维基的长条目，存一份到 `tests/corpus/local/`）解析 + 算样式 ≤ 80 毫秒（release，插电）。

---

## 1b 文字和排版

### 8. 平台接口

内核定义 `platform` 接口，至少包括：按 `font-family` 列表和粗细、斜体、语言找字体；字体度量（上升、下降、行距、x 高、下划线位置和粗细）；把一段文字整形成字形（字形号、前进宽度、对应的字符范围），含字体回退；整段文字的断行位置；把字形画成灰度遮罩；编码转换。图片解码和网络在 1c 加。

- **测试平台**（在内核里，单元测试用）：每个字形都是 1em 见方，上升 0.8em、下降 0.2em（和 WPT 的 Ahem 字体一样）；空格后可断，任意两个中日韩字符之间可断，中文的后置标点（`，。、；：？！）」』` 等）前不可断。所有排版快照测试都用它，结果和电脑上装了什么字体无关。
- **Windows 平台**（`qinglan-win`，DirectWrite）：
  - 字体：通用字体族的对应：`serif` → Times New Roman，`sans-serif` → Segoe UI，`monospace` → Consolas，`system-ui` → Segoe UI；中日韩文字靠 DirectWrite 的系统回退，按元素的 `lang` 传语言（zh-Hans、zh-Hant、ja、ko），这样简体、繁体、日文字形各自选对。CSS 里写中文字体名（"微软雅黑"、"宋体"）也要能找到。
  - 整形：`IDWriteTextAnalyzer` 分析文字类型、取字形和位置；回退用 `IDWriteFontFallback::MapCharacters`。
  - 断行：对整个行内排版段落的文字调一次 `AnalyzeLineBreakpoints`（UAX #14），再叠加 CSS 的 `word-break`、`overflow-wrap`、`white-space`。
  - 画字：`IDWriteGlyphRunAnalysis` 出灰度遮罩，横向位置量化成 4 档；字形缓存 LRU，≤ 8 MB。缺粗体、斜体时用 DirectWrite 的模拟。
  - 整形缓存：按（字体、字号、文字）缓存整形结果，LRU，计入内存预算。

### 9. 盒子树

- 按 `display` 生成盒子；匿名块盒（CSS2 9.2.1.1，块元素放在行内元素里时要拆开）；`display: contents`；空白文字节点的处理。
- **退路**：`flex`/`grid`/`table*` 及其 inline 形式当 `block`/`inline-block`；`float` 忽略（元素留在正常流里）。

### 10. 排版

- 坐标用整数，单位 1/64 CSS 像素（和 Blink 一样，避免浮点误差累积）。
- **块排版**：宽度（CSS2 10.3.3）、高度（10.6.3）、最小最大值（10.4、10.7）、外边距合并（8.3.1，包括父子、兄弟、空块）、`box-sizing`、百分比（含块高度为 auto 时，百分比高度当 auto）、`overflow: hidden/clip` 裁剪（`auto`/`scroll` 第 1 层当 `visible`，内容不丢）。
- **行内排版**：行盒、行内盒的内边距/边框/外边距、`vertical-align`、`line-height`（`normal` = 主字体的上升 + 下降 + 行距）、行的支柱（strut）、`text-indent`、`text-align`（含 `justify`：西文在空格处拉开，中文在字和字之间拉开）、`<br>`、`inline-block`（收缩适应宽度）、`text-transform`、`letter-spacing`、`word-spacing`、下划线和删除线。
- **空白处理**（css-text-3 第 4 章）：六种 `white-space` 模式。源码里两个中日韩字符之间的换行直接去掉，不变成空格（css-text-3 4.1.3；Firefox 这样做，Chrome 不做，记进 decisions.md）。
- **替换元素**：`<img>` 按固有尺寸；有 `width`/`height` 属性时提前按宽高比占位；`srcset`/`sizes` 按设备像素比和宽度挑图；图片没到时按 0 处理，到了再重排（1c 加节流和滚动锚定）。
- **列表**：标记在外（放在第一行左边）和在内；`disc`、`circle`、`square`、`decimal`、`lower/upper-alpha`、`lower/upper-roman`、`cjk-decimal`、`none`；`<ol start reversed>`、`<li value>`。
- **定位**：`relative`（偏移）；`absolute`（包含块为最近的定位祖先的内边距框，CSS2 10.3.7、10.6.4，含静态位置和收缩适应）；`fixed`（以视口为准，滚动时不动）。绘制顺序：先正常流，再按文档顺序画定位元素；`z-index` 为负的画在正常流之前。完整的层叠顺序留到第 2 层。
- 画布背景：根元素或 `body` 的背景铺满整个画布（CSS2 14.2）。
- **表单控件**：按默认样式画成带边框的行内块，显示按钮文字、输入框的值或灰色的 `placeholder`、下拉框的第一项、文本框的内容。不能操作。
- **`iframe`、`video`、`audio`、`canvas`、`object`、`embed`**：按尺寸画灰色占位框，写上类型；`iframe` 的框可以点，点了打开它的地址。
- `<details>` 一律当作展开（第 1 层不能点开收起）。
- **工作量计量**：和 qingpdf 一样，排版的每一步都计数，每页上限（实现时定，写进 decisions.md）。超了就停止排版，已经排好的部分照常显示，顶部提示"页面太复杂，只显示了一部分"。

### 11. 命令（1b）

| 命令 | 作用 |
|---|---|
| `qinglan-cli layout <文件> [--width 1280] [--test-font]` | 打印盒子树：每个盒子的类型、位置、大小；行盒和文字片段 |

### 12. 排版测试网页

`tests/pages/` 下每个特性一个手写网页，配一份预期的盒子树输出（用测试平台），第一次生成后逐个人工核对。至少覆盖：外边距合并的各种情况、行内排版和 `vertical-align`、六种 `white-space`、中文断行和标点、中英混排、中文之间的换行、列表（嵌套、`start`、`reversed`）、三种定位、图片有尺寸和没尺寸、`srcset`、`@media` 三种宽度、`var()` 和 `calc()`、`@layer` 和 `!important`、老式表现属性、`<noscript>`、flex/浮动/表格的退路、GBK/Big5/UTF-16 编码和 `<meta>` 写在后面的情况。

### 1b 验收

1. `cargo test` 全过、clippy 无警告，贴输出。
2. 所有排版快照测试通过；报告里说明每个预期结果是怎么核对的。
3. 用 Windows 平台跑 `layout`：中文维基长条目和一个 GBK 老网页能排完，没有方块字（找不到字形的情况为 0，或逐个说明原因）。
4. 性能：中文维基长条目，排版（不含解析）≤ 100 毫秒（release，插电，宽 1280）。

---

## 1c 网络、绘制、窗口

### 13. 网络（WinHTTP）

- http/https；开 HTTP/2；TLS 和证书校验用系统的。证书有问题就显示错误页，第 1 层不提供"继续访问"。
- 自动用系统代理设置。重定向最多 10 次。请求头 `Accept-Encoding: gzip, deflate`，由 WinHTTP 解压；解压后的大小同样受上限约束（防压缩炸弹）。
- 第 1 层不收发 Cookie。Referer 按 `strict-origin-when-cross-origin`。User-Agent：`Mozilla/5.0 (Windows NT 10.0; Win64; x64) qinglan/0.1`（记进 decisions.md）。
- 上限：HTML、CSS 每个 ≤ 16 MB，图片每张 ≤ 32 MB；连接超时 15 秒，总时间 60 秒。并发每个主机 ≤ 6，总共 ≤ 16。网络在工作线程上跑，界面线程永远不等网络。
- `data:` 网址：子资源可以用（自己解 base64 和百分号编码）；不能作为整页打开。
- `file:` 网址：只能从地址栏或命令行打开；网页里的 `file:` 链接和子资源一律不加载。
- https 页面里的 http 子资源（CSS、图片）不加载。
- 非 HTML 的响应：`text/plain` 当预排版文字显示；图片单独显示；其他类型显示"第 1 层暂不支持下载"，附上网址。
- `mailto:` 先弹窗确认，再交给系统的邮件程序；`javascript:` 和其他协议忽略，状态栏提示。
- 内存里的响应缓存（给前进后退用）：LRU，≤ 16 MB，计入内存预算。

### 14. 录制和回放

- `qinglan-cli record <网址> <目录>`：正常加载一次，把每个响应（状态码、响应头、内容）存成普通文件，再存一个索引。
- 所有命令（包括 `qinglan.exe`）加 `--replay <目录>`：只从目录取响应，目录里没有的网址一律返回 404，绝不联网。
- 索引用文本格式，可以手工编辑，方便构造恶意响应。

### 15. 图片（WIC）

- JPEG、PNG、GIF（第一帧）、BMP、ICO、WebP（系统没装 WebP 解码器时显示为坏图）。SVG 第 1 层不显示，有 `alt` 就显示 `alt` 文字。
- 先只读文件头拿尺寸；像素数超过 1 亿的不解码，当坏图处理。
- 按显示尺寸（设备像素）解码，大图直接缩小解码，不先解成原图。
- `loading="lazy"` 的图片，以及离视口两屏以外的图片，等滚近了再请求、再解码；离视口四屏以外的解码结果可以丢掉。解码后的图片缓存 ≤ 24 MB。

### 16. 绘制

- 内核从排版结果生成显示列表：底色矩形、边框（第 1 层所有边框样式都画成实线，记进 decisions.md）、图片、文字（字形号、位置、颜色）、装饰线、裁剪区域；`fixed` 元素单独一份列表，不随滚动。
- 内核自己合成像素：每帧只画视口大小的 BGRA 缓冲区，按位置索引只取和视口相交的项目；图片用双线性缩放；文字按遮罩上色混合。最后通过平台接口贴到窗口上。
- 高 DPI：按显示器 DPI 感知（per-monitor v2）；排版用 CSS 像素，绘制用设备像素。
- 滚动时重画视口；如果达不到每帧 16 毫秒，再做"平移旧画面、只画新露出的部分"。

### 17. 窗口（`qinglan.exe`）

- 上面：后退、前进、刷新/停止三个按钮和地址栏（系统的 EDIT 控件，中文输入法直接能用）。下面：状态栏（鼠标指向链接时显示地址、加载进度、http 网页显示"不安全"）。标题栏："网页标题 - 轻览"。
- 地址栏：带协议的直接打开；像主机名的（有点或者是 `localhost`，没有空格）补 `https://`；其余当搜索词，用 `https://html.duckduckgo.com/html/?q=`。
- 链接：鼠标变手形，状态栏显示地址；点击打开；Ctrl+点击、中键、`target="_blank"` 在新窗口打开（启动一个新进程）；`#片段` 滚到对应的 `id`/`name`。
- 快捷键：Ctrl+L/F6 到地址栏，Enter 打开，Esc 停止，Alt+←/→ 前进后退，F5/Ctrl+R 刷新，Ctrl+N 新窗口，空格/Shift+空格、PageUp/PageDown、Home/End、方向键、鼠标滚轮（按系统设置的行数）。
- 前进后退：每个窗口在内存里记历史，回去时恢复滚动位置；有缓存的从缓存重新解析，不重新下载。
- 页面加载：HTML 和 `<head>` 里的 CSS 到齐后开始显示（CSS 最多等 5 秒，超时就先不用它显示）；图片陆续到达时重排，重排最多每 100 毫秒一次，并保持视口里第一个元素的位置不跳（简单的滚动锚定）。
- 错误页（网络、证书、超时等）是内置的 HTML，由我们自己的内核显示。HTTP 4xx/5xx 带内容的就显示网站给的内容。
- 每次加载、排版、绘制都包在 `catch_unwind` 里：万一出了 bug，显示"这个网页触发了轻览的错误"，窗口不关。
- 栈溢出 `catch_unwind` 接不住，进程会直接退出。所以两个 exe 的主线程栈都设成 8 MB（链接参数 `/STACK`），跑页面的其他线程也一样；各处递归另外还有层数上限（1a 审查发现默认 1 MB 栈只有约 2 倍余量）。
- 打开新页面时，旧页面的 DOM、样式、排版全部释放。
- 命令行：`qinglan.exe [网址]`；`qinglan.exe --bench <网址> [--replay <目录>] [--width W --height H]`：加载、从头滚到尾，打印各阶段耗时和峰值内存后退出。
- 界面文字中英双语，跟系统语言走。

### 18. 命令（1c）

| 命令 | 作用 |
|---|---|
| `qinglan-cli render <网址或文件> -o out.png [--width 1280] [--height 1000 \| --full]` | 渲染成 PNG（用 WIC 编码） |
| `qinglan-cli record <网址> <目录>` | 录制 |
| `qinglan-cli bench <网址> --replay <目录>` | 各阶段耗时（解析、样式、排版、绘制）和内存 |

### 19. 测试

- **真实网页**：录至少 30 个到 `tests/corpus/local/`（不提交），网址、类别、录制日期写进 `tests/corpus/PAGES.md`（提交）。类别至少包括：中文维基（含一个长条目）、英文维基、中文博客、中文新闻正文（2–3 个）、技术文档（MDN、Rust、Python）、GitHub 仓库首页、Stack Overflow 问答页、Hacker News（表格排版，看退路的效果）、GBK 老网站、Big5 网站、日文网站、政府网站、静态博客（Hugo/Jekyll 一类）。不录需要登录或含个人信息的页面。
  - 每个页面用我们的 `render` 和 Edge 无头模式（关掉 JS：`--blink-settings=scriptEnabled=false`，同样宽 1280）各截前三屏，拼成并排对比图，放 `tests/out/`（不提交）。
  - 汇总表：能否打开、文字是否完整且顺序正确、有没有乱码或方块、缺几张图、耗时、内存。
- **恶意网页**（在 Rust 测试里生成；需要真实网络行为的，用测试里起的本地 HTTP 服务器，走 WinHTTP）：10 万层嵌套、200 万个元素、10 MB 没有空格的一段文字、10 万行 CSS、超长选择器、`var()` 指数展开、`@import` 循环和 1000 个 `@import`、`calc()` 嵌套 1 万层、10 万张图片、文件头声称 10 万×10 万像素的 PNG、1 GB 的 gzip 炸弹、重定向循环、永远不结束的响应、单个元素 10 万个属性、坏的 UTF-8 和 GBK、空字节、`font-size: 1e9px`、巨大的正负外边距、1 万层嵌套的行内元素、偏移 1e9 的定位元素。每个都要 5 秒内出结果，不 panic，峰值 Private Bytes ≤ 200 MB。

### 1c 验收（也是第 1 层的验收）

1. `cargo test` 全过、clippy 无警告，贴输出。
2. 真实网页：汇总表和对比图。每个页面都能打开，文字完整、顺序对，没有乱码和方块，图片都在。和 Edge 的差别逐个说明，应该只剩"第 2 层的特性"。
3. 恶意网页：汇总表（耗时、峰值内存、结果）。
4. 性能和体积（release，插电），贴数字：
   - `qinglan.exe` ≤ 1.5 MB（给后面几层留空间），只依赖系统 DLL
   - 启动到能输入网址 ≤ 0.3 秒
   - 中文维基长条目（回放）：HTML 和 CSS 到齐后 ≤ 200 毫秒画出第一屏
   - 1920×1080 下滚动，每帧中位数 ≤ 16 毫秒、95 分位 ≤ 33 毫秒；DPI 100% 和 150% 各测一次
   - 中文维基长条目从头滚到尾后，Private Bytes ≤ 50 MB；真实网页库的中位数和最大值也报出来
5. 手工检查清单（给我和用户）：打开 exe，输网址，点链接，前进后退，改窗口大小，在高 DPI 显示器上看。
6. 审查意见和处理结果。
7. `docs/decisions.md`。

---

## 不做（第 1 层）

- JS
- 浮动、flex、grid、表格的真正排版（用退路）、完整的 `z-index` 层叠顺序
- `::before`/`::after`、背景图、渐变、圆角、阴影、透明度、变形、动画、过渡
- 网页字体、SVG、彩色 emoji（先显示成单色）、竖排、从右往左的文字
- `:hover` 等交互状态、文字选择和复制、页内查找、缩放
- 表单的输入和提交（只显示样子）
- `iframe` 的内容、视频音频、`canvas`（显示占位框）
- 内部滚动区域（`overflow: auto/scroll` 当 `visible`）
- 标签页、历史记录保存、书签、下载、Cookie、磁盘缓存、打印、设置、记住窗口位置
