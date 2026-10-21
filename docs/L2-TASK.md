# 第 2 层：现代排版

做完后，大部分网站不靠 JS 的部分样子就对了：维基百科、GitHub、MDN、新闻站的导航、侧栏、信息框、表格、图标都在该在的位置。

还是不跑 JS。做法和第 1 层一样：每一步交给一个人做，我检查后再开下一步；审查放在 2b、2d、2e 之后，各做一次（2d 有字体解码，是不可信输入）。

## 分步

| 步 | 内容 | 为什么排在这里 |
|---|---|---|
| 2-0 | 第 1 层收尾：滚动卡顿、启动偶尔慢、内存余量 | 第 2 层会让排版更重、内存更紧，先把底子打好 |
| 2a | 浮动、表格、`::before`/`::after`、层叠顺序 | 老网站和信息框、Hacker News 都靠表格和浮动 |
| 2b | flex、grid | 现代网站的主体布局 |
| 2c | 绘制效果：背景图、渐变、圆角、阴影、透明度、2D 变形、边框样式 | 需要一个自己写的路径光栅器，2e 的 SVG 复用它 |
| 2d | 文字：网页字体、彩色 emoji、从右往左、竖排、注音 | 字体解码是不可信输入，单独审查 |
| 2e | SVG、内部滚动区域、`position: sticky`、表单控件外观 | 用 2c 的光栅器 |

## 体积、内存、速度的预算

- **exe**：第 2 层做完 ≤ 2.4 MiB（2,516,582 字节）。每步报增量，参考分配：2a +80 KB、2b +100 KB、2c +100 KB、2d +150 KB（Brotli 字典约 120 KB）、2e +100 KB。哪一步超了先停下说明。
  - **2026-10-20 改：第 2 层上限放宽到 2.6 MiB（2,726,297 字节）。** 2c-1 之后是 2,183,168 字节，还剩 543,129 字节。新分配：2c-2 +60 KB、2d +200 KB、2e +150 KB，剩下约 130 KB 留给意外。不再单独做压体积的步骤：每一步写完代码、提交之前，自己用链接映射看体积，超了先在本步内收（共用代码、少用泛型、冷代码不内联），还超就停下报告；第 2 层最后统一压一次。
- 2a 结束（2a-3 之后，2026-10-17）：1,927,168 字节，比 2a 开始时还小；2b–2e 还剩约 589 KB，原分配不变。2a-3 加了自己写的内存分配器（`qinglan-win/src/sys/heap.rs`）和 `-inline-threshold=100`，见 decisions.md。
- **内存**：中文维基长条目滚到底后 Private Bytes 仍然 ≤ 50 MB（5 次新进程的最大值）；真实网页库报中位数和最大值，超过 80 MB 的页面逐个查原因。
- **速度**：冷启动首屏 ≤ 200 ms（中文维基、两个 GitHub 页面，各 5 次中位数）；滚动每帧中位数 ≤ 16 ms、95 分位 ≤ 33 ms。
- 每一步都要报这三项的前后对比；量法同第 1 层（插电、新进程、新旧程序交替跑）。

## 测试方法（第 2 层新增）

- **WPT reftest**：Web Platform Tests（BSD-3）里的参考测试，测试页和参考页都用我们的引擎、测试平台（每个字 1em 见方，就是 WPT 用的 Ahem 字体的样子）渲染，比较像素。每一步跑对应目录的子集，下载到 `tests/wpt/`（只放用到的文件，带许可证），报通过率。没通过的按原因分两类：依赖还没做的特性（写明哪一步做），或者是 bug。**bug 类必须修完，或者逐条写明为什么不修。**
- **手算快照**：和第 1 层一样，每个新特性一个小网页，手算关键数字。
- **真实网页**：每步结束重跑 49 页的对比图（`tools/corpus_run.py`），更新 `tests/corpus/RESULTS.md`。第 2 层结束时，"只差第 2 层特性"那一类应该清空：剩下的差别逐条说明是第 3、4 层的事还是 bug。

---

## 2-0 第 1 层收尾

1. **滚动卡顿**：现在图片到达后整页重排，在界面线程上约 200 ms，滚动中会卡一下。
   - 有 `width`/`height` 属性或 `aspect-ratio`、尺寸已经占好的图片，到达后只重画，不重排。
   - 其余需要重排的，用户正在滚动时推迟到最后一次滚动输入后 150 ms 再排，多张图合并成一次；滚动锚定照旧。
   - 验收：中文维基 `--bench` 5 次，每次滚动最长一帧 ≤ 50 ms（现在 270–310 ms）。`--bench` 的滚动过程要像真人一样边滚边让图片到达。
2. **启动偶尔 0.5 秒**：5 次里会有 1 次。查清原因（Defender 扫描新 exe、系统字体缓存、别的），能修就修；修不了写清原因和条件。
3. **内存余量**：DOM 属性值放进一个共享的池（约省 2 MB），以及量出来的其他大头。验收：中文维基 5 次新进程最大值 ≤ 46 MB，给第 2 层留 4 MB。

## 2a 浮动、表格、生成内容、层叠顺序
为了省用量，2a 分两小步：**2a-1** 先在空闲机器上复测样式速度（2-0 后可能慢了约 12%，超过 5% 就修）、修 WebP 没有解码器时变空白的问题、搭 WPT reftest 的运行框架，再做浮动和生成内容；**2a-2** 做表格和层叠顺序，重跑真实网页对比。

- **浮动**（CSS2 9.5、9.5.1 的 9 条规则、9.5.2 `clear`）：左右浮动、行盒缩短、浮动在行内内容中间、块格式化上下文的根避开浮动、清除间距（clearance）、`display: flow-root`。取代第 1 层"浮动当普通块"的退路。
- **表格**（CSS2 第 17 章，宽度分配参考 css-tables-3 草案和主流浏览器的实际做法）：表格、行组、行、单元格、列、列组、标题；匿名表格对象（17.2.1）；自动布局和固定布局（17.5.2）；`rowspan`/`colspan`；`border-collapse` 和边框冲突规则（17.6.2）；`border-spacing`；单元格的 `vertical-align`；`empty-cells`；`caption-side`；HTML 的表格属性（`cellpadding`、`cellspacing`、`border`、`width`、`align`、`valign`、`bgcolor`，中文老网站大量使用）。取代第 1 层"表格当普通块"的退路。
- **生成内容**（CSS2 第 12 章）：`::before`/`::after` 的 `content`：字符串、`attr()`、`counter()`/`counters()`、引号、`url()` 图片；`counter-reset`/`counter-increment`/`counter-set`；`quotes`；`::marker` 的 `content`。
- **层叠顺序**（CSS2 附录 E）：层叠上下文、`z-index`、完整的绘制顺序；点击检测按绘制顺序从上往下找。
- WPT 子集：`css/CSS2/floats`、`floats-clear`、`tables`、`generated-content`、`zindex`；`css/css-tables` 里不依赖还没做的特性的部分。
- 真实网页：Hacker News、两个 GBK 老网站、维基信息框，和 Edge 对比明显改善。

## 2b flex 和 grid
为了省用量，2b 分两小步：**2b-1** flex，**2b-2** grid，并重跑真实网页对比；2b-2 之后做第 2 层的第一次审查（2a + 2b）。

- **flex**（css-flexbox-1 全部）：方向、换行、`order`、`flex-grow/shrink/basis`、`min-width: auto` 的自动最小尺寸、固有尺寸、`justify-content`、`align-items/self/content`、基线对齐、多行、`gap`。
- **grid**（css-grid-1 主体，加 `gap`）：显式和隐式轨道、`fr`、`repeat()`（含 `auto-fill`/`auto-fit`）、`minmax()`、命名线和命名区域、自动放置（含 `dense`）、对齐、轨道尺寸算法（11.5–11.8）。不做 subgrid、masonry。
- 取代第 1 层 flex/grid 当普通块的退路。
- WPT 子集：`css/css-flexbox`、`css/css-grid` 里不依赖还没做的特性的部分。
- 真实网页：维基（Vector 皮肤）、GitHub、MDN、新闻站导航应该接近 Edge。注意中文维基首屏：Vector 用 flex/grid，排版会变重，200 ms 的首屏要守住。

## 2c 绘制效果
为了省用量，2c 分两小步：**2c-1** 路径光栅器、背景图和渐变、圆角、边框样式、outline；**2c-2** 阴影、透明度、2D 变形（含点击检测），重跑真实网页对比。

- **路径光栅器**（自己写）：直线和贝塞尔曲线、非零和奇偶填充规则、抗锯齿（按覆盖率）、描边（线宽、端点、连接、斜接上限、虚线）。2e 的 SVG 复用。
- **背景**：`background-image`（`url()`，多层）、`position`/`size`/`repeat`/`origin`/`clip`；`linear-gradient`、`radial-gradient`、`conic-gradient` 及其 `repeating-` 形式。`background-attachment: fixed` 当 `scroll` 处理（记进 decisions.md）。
- **圆角**：`border-radius` 用在背景、边框、`overflow` 裁剪上。
- **边框样式**：`dashed`、`dotted`、`double`、`groove`、`ridge`、`inset`、`outset`；`outline`。
- **阴影**：`box-shadow`（外阴影、内阴影、扩展），`text-shadow`；模糊用三次盒式模糊近似高斯模糊。
- **透明度**：`opacity` 用离屏层合成；离屏层大小有上限，超了就不做透明直接画（记进 decisions.md）。
- **2D 变形**：`transform`（`translate`、`scale`、`rotate`、`skew`、`matrix`）、`transform-origin`；绘制和点击检测都按变形走；3D 变形当 2D 处理（只取 2D 部分）。很多网站用 `translate(-50%, -50%)` 居中，没有它位置会错。
- `filter`、`mix-blend-mode`、`clip-path`、`mask` 不做（记进"不做"）。
- WPT 子集：`css/css-backgrounds`、`css/css-transforms`（2D）、`css/css-color` 里不依赖还没做的特性的部分。

## 2d 文字
为了省用量，2d 分两小步：**2d-1** 网页字体（`@font-face`、自己写的 inflate、Brotli、WOFF2 解码，DirectWrite 从内存加载）；**2d-2** 彩色 emoji、从右往左、竖排、注音。2d 之后做第 2 层的第二次审查，重点是字体解码器（不可信输入）。

2d-1 之后（2026-10-20）：exe 2,479,616 字节，比 2d-1 的预算多 57 KB；第 2 层上限 2,726,297 还剩 246,681。重新分配：**2d-2a**（彩色 emoji、从右往左）+40 KB，**2d-2b**（竖排、注音）+60 KB，2e +120 KB，余约 25 KB。竖排先只做块和行内排版里的竖排；flex、grid、表格放在竖排里时按横排排（记进 decisions.md）。
- **网页字体 `@font-face`**：TTF、OTF、WOFF（自己写 inflate）、WOFF2（自己写 Brotli 解码器和 WOFF2 的表变换还原）；用 DirectWrite 从内存加载（`IDWriteInMemoryFontFileLoader`）；`unicode-range`、粗细和斜体的范围；`font-display`：最多等 3 秒，超时先用系统字体显示，字体到了再重排。
  - 上限：单个字体解压后 ≤ 8 MB，每页所有网页字体解压后合计 ≤ 16 MB，计入内存；超了就用系统字体。
  - WOFF/WOFF2 解码器是我们自己写的，**所有边界都按不可信输入处理**；字体本身交给 DirectWrite 解析。
- **彩色 emoji**：DirectWrite 的 `TranslateColorGlyphRun`（COLR/CPAL），Segoe UI Emoji 显示成彩色。
- **从右往左的文字**：UAX #9 双向算法（用 DirectWrite 的 `AnalyzeBidi`），`direction`、`unicode-bidi`，阿拉伯文、希伯来文段落正确排列。
- **竖排**（css-writing-modes-3）：`writing-mode: vertical-rl`/`vertical-lr`、`text-orientation`、`text-combine-upright`（纵中横）；竖排和横排嵌套时的正交流。中文竖排是"中文做到最好"的一部分。
- **注音**（css-ruby-1 基本部分）：`<ruby>`/`<rt>`/`<rp>`，注音在上方、`ruby-position`；拼音和日文假名。青空文库页面要正确显示。
- WPT 子集：`css/css-fonts`（`@font-face` 部分）、`css/css-writing-modes`、`css/css-ruby` 里不依赖还没做的特性的部分；这一步起 WPT 可以真正加载 Ahem 字体。

## 2e SVG、内部滚动、sticky、表单外观
为了省用量，2e 分两小步：**2e-1** SVG（体积 +80 KB）；**2e-2** 内部滚动、`position: sticky`、表单控件外观，顺带横向滚动条和 Shift+滚轮（+40 KB）。2e 之后是第 2 层收尾（内存、体积、遗留问题）和第三次审查。

- **SVG**（自己写，复用 2c 的光栅器）：`<img src=*.svg>`、CSS 背景里的 SVG、网页里直接写的 `<svg>`（图标大量这样用）。支持 `svg`/`g`/`use`/`defs`/`symbol`、`path`（全部命令含圆弧）、基本图形、填充和描边、`fill-rule`、`opacity`、`transform`、`viewBox` 和 `preserveAspectRatio`、`currentColor`、线性和径向渐变、简单的 `clipPath`、简单的 `<text>`；样式用我们自己的 CSS 引擎算。不做：滤镜、蒙版、动画、`foreignObject`。SVG 文件同样是不可信输入（元素数、`use` 展开、嵌套深度都有上限）。
- **内部滚动区域**：`overflow: auto/scroll` 有滚动条，滚轮和键盘滚动鼠标所在的区域；`overflow: hidden` 的区域用户滚不动。
- **`position: sticky`**：真正的吸顶，取代第 1 层"当 relative"的退路。
- **表单控件外观**：按钮、输入框、复选框、单选框、下拉框按 Windows 11 的样子画；仍然不能操作（第 3 层）。
- WPT 子集：`svg/` 里静态渲染的部分、`css/css-overflow`、`css/css-position`（sticky）。

---

## 第 2 层的验收

1. `cargo test --workspace` 全过、clippy 无警告。
2. 每一步的 WPT 通过率和没通过的分类；bug 类全部修完或逐条说明。
3. 真实网页：49 页重跑，"只差第 2 层特性"一类清空，剩下的差别逐条说明。
4. 体积、内存、速度：按上面的预算，贴出数字。
5. 三次审查（2b、2d、2e 之后）的意见和处理结果。
6. `docs/decisions.md`。

## 不做（第 2 层）

- JS 以及只靠 JS 才能显示的内容（懒加载图片的 `data-src` 等）
- 表单的输入和提交、文字选择、查找、缩放、标签页（第 3 层）
- `filter`、`mix-blend-mode`、`clip-path`、`mask`、3D 变形、动画和过渡、`@container`、`:has()`、subgrid、masonry
- SVG 滤镜、蒙版、动画、`foreignObject`
- 视频、音频、`canvas`、`iframe` 的内容（仍然是占位框）

## 进度

- 2b-1（flex）完成（2026-10-17）：`css-flexbox` WPT 647 / 951，exe 1,984,000（+56,832，比 2b-1 的 +50 KB 多 6.8 KB，2b 总分配 +100 KB 内），zhwiki-rec 内存最大 46.51 MB，首屏中位数 113 / 170 / 149 毫秒；详见 `docs/decisions.md` "2b-1"。

- 2b-2（grid）完成（2026-10-17）：`css-grid` WPT 554 / 1573，exe 2,138,624（+154,624，**超出 2b 预算 111,624**：grid 实现比分到的 43 KB 大；2c–2e 的体积要重新分配，总上限 2,516,582 还剩 377,958），zhwiki-rec 内存最大 47.86 MB，首屏中位数 146.6 / 167.5 / 146.2 / 34.4 毫秒；首屏优先进入列 flex、行 flex 和 grid；49 页重跑见 `tests/corpus/RESULTS.md` 第 6 节；详见 `docs/decisions.md` "2b-2"。
- 2b-3（grid 之后的首屏和体积，2026-10-20）：zhwiki-rec 首屏中位数 143.8 → 120.5 毫秒（`qinglan-cli bench`；原因是页面第一次排对，不是 grid 的最小内容宽度；首屏优先不再排看不见的脱离文档流盒子）；exe 2,150,400 → 2,012,160（排序去重、`inline-threshold=45`、`inlinehint-threshold=150`），2b 预算 2,027,000，余 14,840，可算进 2c；详见 `docs/decisions.md` "2b-3"。
- 2b 结束（2026-10-20，我复核后）：exe 2,012,160 字节，第 2 层总上限 2,516,582 还剩 504,422（约 493 KB），2c–2e 原分配 350 KB 不变，余量留着。保留 `inlinehint-threshold=150`：去掉它能让整页排版快约 3–5%，但会超预算约 20 KB；首屏和滚动不受影响，所以保留（2026-10-20 定）。修了一个测试互相删文件导致的偶发失败（`crates/qinglan-cli/tests/render.rs`）。

- 2c-1（路径光栅器、背景、渐变、圆角、边框样式、outline，2026-10-20）完成：`css-backgrounds` WPT 198 / 212（93.4%），bug 类 5，`css-images` 58 / 126（46.0%），bug 类 0（bug 类每条都在 `tests/wpt/notes.txt` 里有说明）；`qinglan.exe` 2,196,992（+184,832，**超出 2c-1 的 +60 KB 约 125 KB**；总上限还剩 319,590，2c-2、2d、2e 要重新分配）；zhwiki-rec 滚到底最大 48.81 MB；滚动光栅 zhwiki-rec 5.0 / 6.9 ms、gh-ripgrep 11.0 / 16.2 ms；**gh-ripgrep 首屏中位数 205–230 ms 没达到 200 ms**，没确认是否噪声；详见 `docs/decisions.md` "2c-1"。 2c-1b（2026-10-20）：`wpt --check` 的三个退步修好（折叠边框的 padding box 缩进、rtl 表格列序，2221 / 2221）；gh-ripgrep 滚动光栅 12.4 → 5.4 ms（旧 5.1，圆环按带填）；`qinglan.exe` 2,196,992 → 2,183,168（仍超 2c-1 预算 109,568）；gh-ripgrep 首屏 ≤ 200 ms、zhwiki-rec 滚到底 ≤ 47 MB 没达到（47.98），详见 decisions.md "2c-1b"。

- 2c-2（阴影、透明度、2D 变形，2026-10-20）完成：`css-transforms` WPT 133 / 146（bug 类 4，都写明原因），`css-color` 的 `opacity` 11 / 12，`text-shadow` 16 / 18（bug 类 1），`css-backgrounds`（加了 `box-shadow`）220 / 237（bug 类 6）；`wpt --check` 2403 / 2403；`qinglan.exe` 2,253,824（+70,656，**超出 2c-2 预算 +61,440 共 9,216 字节**，总上限还剩 472,473；留给第 2 层最后统一压）；zhwiki-rec 滚到底最大 47.33 MB，首屏中位数 164 ms，滚动中位数 6.7 ms；gh-ripgrep 首屏中位数 232 ms 仍没达到 200 ms；合成的阴影压力页滚动 73 ms（不达标，文字阴影模糊没缓存）；49 页重跑：和 Edge 一致 13、只差后面步骤 36、bug 0（`tests/corpus/RESULTS.md` 最后一节）；详见 `docs/decisions.md` "2c-2"。

- 2c-3（样式速度、超过层数上限的组、文字阴影缓存，2026-10-20）完成：gh-ripgrep 样式阶段 167 → 约 80 ms（`parse_sheets` 115–140 → 先读规则 25–31 + 转规则 21–24：不给要丢掉的规则的选择器和声明块分词；分词器改按字节），49 页 `style` 转储前后逐字节相同；首屏中位数（插电，旧新交替）gh-ripgrep 201 → 138 ms、gh-rust 180 → 131、zhwiki-rec 164 → 152（收尾时电脑变成用电池，只留旧新对比，见 decisions.md）；超过层数上限的组不再丢（深度超 6 直接画，旋转层太大分带画）；模糊文字阴影按整个 run 缓存，压力页只开文字阴影 33.9 → 13.8 ms，**全部特效的压力页 131 → 114 ms，没到 16 ms**（盒阴影、旋转缩放、透明度每帧重画，要缓存整张卡片，留给后面）；`qinglan.exe` 2,268,672（+14,848）；zhwiki-rec 滚到底最大 47.11 MB；`wpt --check` 2403 / 2403；详见 `docs/decisions.md` "2c-3"。

- 2d-1（网页字体，2026-10-20）完成：自己写的 inflate、Brotli（含静态字典，exe 里存 58,898 字节）、WOFF、WOFF2 解码器，Brotli 官方向量 37 个全过，坏输入/超大/炸弹都在 1 秒内干净失败（`hostile.rs` 三个整页用例 224 / 47 / 52 ms）；`@font-face`（`src`、`unicode-range`、粗细/样式/拉伸范围、`font-display`）、字体匹配 5.2、DirectWrite 内存加载；WPT `css/css-fonts`（`--win`）32 / 51，bug 类 3（各有说明），`wpt --check` 2435 / 2435；zhwiki-rec 滚到底最大 47.41 MB，首屏中位数 140.9 / gh-ripgrep 189.6 ms；`qinglan.exe` 2,479,616（**+210,944，超出 2d-1 预算 57,344**，留给第 2 层最后压）；详见 `docs/decisions.md` "2d-1"。

- 2d-2a（彩色 emoji、从右往左，2026-10-20）完成：Segoe UI Emoji 的 COLR 彩色字形（`TranslateColorGlyphRun`，各层叠成一张预乘 BGRA，进字形缓存；ZWJ、肤色、VS16、keycap 序列是一个簇；位图和 SVG 字体没做）；UAX #9 双向文字（`AnalyzeBidi`）、`direction`、`unicode-bidi` 六个值、`dir`（含 `auto`、`<bdi>`、`<bdo>`）、逻辑属性随 `direction` 翻转、rtl 的块（start/end 对齐、过约束外边距、列表标记、`float: inline-start`）、折叠边框表格镜像；没有 RTL 字符也没有会重排的盒的段落不做任何分析（0 次）；WPT：`css-writing-modes`（bidi 相关）83 / 98、`CSS2/bidi-text` 69 / 103（bug 类 34：被重排切开的行内盒的边，各有说明）、`css-text/text-align` 51 / 85；真实网页：阿拉伯文维基、英文维基 Emoji 条目对 Edge；详见 `docs/decisions.md` "2d-2a"。
  2d-2a 的数据：`qinglan.exe` 2,517,504（+37,888，预算 +40 KB 内）；zhwiki-rec 滚到底最大 48.28 MB，首屏中位数 129.1 ms，gh-ripgrep 130.7 ms（插电，Emulator.exe 约 9% CPU）；`wpt --check` 2635 / 2635。没做：被重排切开的行内盒的边（WPT bug 类 34）、`plaintext` 在 `<br>` 后重新定方向、`text-align-last`、`::marker` 的 `unicode-bidi`、词中换样式时的连写。

- 2d-2b（竖排、注音，2026-10-21）完成：先修了 `bad_utf8`（154 → 109 MB，限额 120；`bidi_levels` 给每个字节分了 8 字节的索引）；竖排按“框架”做（`layout/vertical.rs`：竖排块是正交流的根，里面用横排代码按逻辑方向排，排完整体换成物理坐标），`writing-mode`（含 `sideways-rl`；`sideways-lr` 只解析）、`text-orientation`、`text-combine-upright`（`all`、`digits`）、`direction: rtl` 在竖排里、body 的值传给根；竖排标点用 `vert` 字形；注音（`ruby`/`rb`/`rt`/`rp`、`ruby-position`）做成匿名 `inline-block` 一对；WPT `css-writing-modes` 439 / 843（bug 类 388，每个一行 `notes.txt`，没有修）、`css-ruby` 23 / 118；flex/grid 在竖排里按横排排（31 个靠“竖排被忽略”碰巧通过的 flex/grid 测试退出通过名单）；`qinglan.exe` 2,567,168（+49,664，预算 +60 KB 内）；zhwiki-rec 滚到底最大 48.42 MB；首屏没有合格的数字（量的时候另一个会话在编译）；详见 `docs/decisions.md` “2d-2b”。

- 2c/2d 审查修复 A（2026-10-21，解码器和放大）：Brotli 上下文映射逆移到前改 O(i) 并加准备工作量预算（984 KB 的炸弹字体 8.5 s → 约 50 ms）；发现 `bench` 不取网页字体，改成取并在 `hostile.rs` 里核对；注音对共用样式并计入生成盒子额度（10 万对 `layout` 356 → 156 MB，2.68 → 0.91 s）；WOFF2 临时拷贝改移动、坐标差超出 16 位算坏字体；彩色字形各层像素和外框设上限。详见 `docs/decisions.md` “2c/2d 审查修复”。

- 2c/2d 审查修复 B（2026-10-21，盒阴影 9 宫格）：遮罩只和圆角、模糊半径有关，不再和盒子大小有关（r1_shadow_only 10.3 → 0.22 s，q5_shadow400 3.64 → 0.36 s，2 Mpx 窗口的卡片每帧 65 → 1.7 ms，1260×3300 的包装层现在有阴影）；画面与旧版逐字节相同。详见 `docs/decisions.md` “提交 B”。
- 2d-2c（竖排页面沿行方向滚动，2026-10-21）完成：滚动位置是沿页面自己的轴走了多远（`ScrollAxis`：横排向下，vertical-rl 向左、起点在右边缘，vertical-lr 向右），范围、图片带、命中、锚定、历史恢复、`--bench` 都沿轴；滚轮、翻页、Space、Home/End、方向键走这条轴；`render --scroll-x`；exe +2,560 字节，zhwiki-rec 内存最大 46.97 MB，横排页滚动帧不变。没做：水平滚动条（竖排页的竖直条显示为禁用）、Shift+滚轮、竖排页的二维索引。详见 `docs/decisions.md` “2d-2c”。

- 2e-1（SVG，2026-10-21）完成：自己的 XML 解析器、场景构建器（样式用我们的 CSS 引擎，SVG 专有属性作为自定义属性条目存在层叠里）、渲染器（复用 `vector/`）；网页里的 `<svg>`、`<img src=*.svg>`、CSS 背景、`data:`、直接打开 `.svg`；`svg`/`g`/`use`/`defs`/`symbol`/`a`/路径（含圆弧）/基本图形/渐变/`clipPath`/简单 `text`/`opacity` 组/`pathLength`；不做滤镜、蒙版、图案、标记、动画、`foreignObject`、`<image>`；WPT `svg/` 78 / 109（71.6%，bug 类 1，其余失败各一行 `limit`），`css/css-backgrounds` 318 / 446，`css/css-images` 59 / 129，`wpt --check` 3188 / 3188；恶意网页 8 个（use 炸弹、100 万个路径命令、深嵌套、极端 viewBox、坏 XML、两万个图标）都 ≤ 1.1 秒、≤ 62 MB；**`qinglan.exe` 2,582,016 → 2,710,528（+128,512，超出本步 +80 KB 约 48 KB；第 2 层上限 2,726,297 只剩 15,769，2e-2 的 +40 KB 放不下，要先压体积或放宽）**；zhwiki-rec 滚到底最大 48.08 MB，首屏中位数 147.1 → 157.9 ms，gh-ripgrep 165.3 → 169.0 ms（+5.8 MB 内存）；遗留：没有固有宽高的 SVG 图片的默认大小算法（110 个背景 WPT 用例）、图片 SVG 里的文字。详见 `docs/decisions.md` “2e-1”。

- 2e-2（内部滚动、sticky、页面横向滚动、表单控件外观，2026-10-21）完成：`overflow: auto/scroll` 的盒子是滚动容器（滚动范围按盒、文字碎片、变形组量；窗格直接画进帧、不开离屏层；引擎画的叠加式细滚动条，不占布局宽度；滚轮给鼠标下最里面还能动的容器再传给外层和页面，拖滑块、点轨道；偏移按 DOM 节点跨重排保留；命中和图片的远近都算偏移）；`position: sticky` 按 css-position-3 3.4 对着最近的滚动口（页面或容器）算，含块限制，画在定位那一轮；横排页宽过视口有横向范围、引擎画的横条、Shift+滚轮，竖排页隐藏原生竖条；表单控件在 `ua.css` 里改成 Windows 11 的样子（1px `#767676` 边、小圆角），复选框/单选框/`progress`/`meter`/`range`/`select` 箭头用栅格器画，`appearance: none`（含 `-webkit-`、`-moz-`）取消；WPT：`css/css-overflow` 18 / 23（bug 类 0）、`css/css-position/sticky` 5 / 5、`html/rendering/widgets` 11 / 19（bug 类 0，8 个失败各一行 `limit`）；`wpt --check` 3218 / 3218（原有 3188 个里 8 个因“空网格的轨道不算可滚动溢出”“min-height: min-content”“叠加式滚动条不占宽”退出名单，各一行 `notes.txt`；另有 28 个以前归“step 2e”的失败改成 `limit (2e-2 leftover, not examined)`，**没有逐个看**）；真实页面：MDN、Rust 文档、GitHub、gov-uk 四页对 Edge（`tests/corpus/RESULTS.md` 最后一节）；`qinglan.exe` 2,710,528 → 2,744,832（+34,304，预算 +40 KB 内；**第 2 层上限 2,726,297 暂时超出 18,535**，接着第 2 层统一压体积）；zhwiki-rec 滚到底最大 49.45 MB（5 次：49.45 / 48.07 / 47.20 / 47.67 / 48.76，余量 0.55 MB），首屏中位数 zhwiki-rec 175.3 ms、gh-ripgrep 133.1 ms，滚动帧中位数 mdn-flex 3.0–3.3 ms、rust-std-vec 3.4–3.9 ms（插电，一个 Emulator.exe 约占一个核 15%，CPU 19–29%）；恶意页面 +3（2 万个滚动容器 259 ms / 53 MB，300 层嵌套 177 ms / 12 MB，1 万个 sticky 125 ms / 24 MB）。**没做**：键盘滚动容器（第 3 层焦点模型）、`scroll-snap`/`scroll-behavior`/`scrollbar-gutter`、从右往左容器的负方向滚动、滚动条悬停变宽、滚动容器里的 `outline`、`select` 弹出和其他控件的操作、窗口里的鼠标操作（按下/拖动/Shift+滚轮）没有在真窗口里手工验证。详见 `docs/decisions.md` “2e-2”。
- 2f-1（体积、内存、首屏，2026-10-21，`Shrink the executable`、`Cut memory`）：`qinglan.exe` 2,744,832 → 2,702,336（目标 2,700,000 差 2,336；上限 2,726,297 还剩 23,961）：排序再去重 −16 KB、`qinglan-app` 改 opt-level "s" 约 −17 KB、去掉 `clamp`/浮点数转文字 −7 KB、下载队列装箱 −7 KB；gh-ripgrep 内存 29.0 → 25.9 MB（原因：SVG 属性当自定义属性存，每个图标复制父元素几百项的表；改成共用；目标 24 没到），gh-rust 26.4 → 22.7，zhwiki-rec 滚到底 47.06–47.42（目标 46.0 没到）；首屏没有可信数字（电池/Emulator 噪声），zhwiki-rec 175 ms 的原因没找到；四项检查通过（wpt 3218 / 3218），52 页渲染 31 页逐字节相同、21 页旧程序自己也不稳定、0 页稳定地不同；详见 `docs/decisions.md` “2f-1”。
