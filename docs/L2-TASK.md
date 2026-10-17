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

- **网页字体 `@font-face`**：TTF、OTF、WOFF（自己写 inflate）、WOFF2（自己写 Brotli 解码器和 WOFF2 的表变换还原）；用 DirectWrite 从内存加载（`IDWriteInMemoryFontFileLoader`）；`unicode-range`、粗细和斜体的范围；`font-display`：最多等 3 秒，超时先用系统字体显示，字体到了再重排。
  - 上限：单个字体解压后 ≤ 8 MB，每页所有网页字体解压后合计 ≤ 16 MB，计入内存；超了就用系统字体。
  - WOFF/WOFF2 解码器是我们自己写的，**所有边界都按不可信输入处理**；字体本身交给 DirectWrite 解析。
- **彩色 emoji**：DirectWrite 的 `TranslateColorGlyphRun`（COLR/CPAL），Segoe UI Emoji 显示成彩色。
- **从右往左的文字**：UAX #9 双向算法（用 DirectWrite 的 `AnalyzeBidi`），`direction`、`unicode-bidi`，阿拉伯文、希伯来文段落正确排列。
- **竖排**（css-writing-modes-3）：`writing-mode: vertical-rl`/`vertical-lr`、`text-orientation`、`text-combine-upright`（纵中横）；竖排和横排嵌套时的正交流。中文竖排是"中文做到最好"的一部分。
- **注音**（css-ruby-1 基本部分）：`<ruby>`/`<rt>`/`<rp>`，注音在上方、`ruby-position`；拼音和日文假名。青空文库页面要正确显示。
- WPT 子集：`css/css-fonts`（`@font-face` 部分）、`css/css-writing-modes`、`css/css-ruby` 里不依赖还没做的特性的部分；这一步起 WPT 可以真正加载 Ahem 字体。

## 2e SVG、内部滚动、sticky、表单外观

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
