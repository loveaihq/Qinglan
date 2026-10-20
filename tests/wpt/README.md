# Web Platform Tests（reftest 子集）

来源：<https://github.com/web-platform-tests/wpt>，提交 `3bcf9ec39dc0f83e20f5665a988c12c0dff5f1e9`（2026-10-16 取，稀疏浅克隆），
许可证 BSD-3-Clause（见 `LICENSE.md`）。只放用到的目录和它们引用的文件：

- `css/CSS2/floats`、`css/CSS2/floats-clear`、`css/CSS2/generated-content`、`css/css-lists`、`css/CSS2/tables`、`css/CSS2/zindex`、`css/css-tables`、`css/css-flexbox`（2b-1）、`css/css-grid`（2b-2，2659 个文件）：被测的测试（后三个只拷了有 `rel=match|mismatch`、没有 `<script>` 的页面和它们引用的参考页、图片、样式表；testharness 和只能人看的页面没拷，所以比 WPT 里的文件数少很多）
- `css/css-backgrounds`、`css/css-images`（2c-1）：只拷了有 `rel=match|mismatch`、没有 `<script>`、也没有 2c-2、2d、2e 和"不做"清单里的特性（阴影、不透明度、变形、字体、书写方向、SVG、表单控件、`mix-blend-mode`、`border-image`、`image-set()`……）的页面和它们引用的文件；筛出来 212 个（背景）和 126 个（图片）。
- `css/css-transforms`（2D，146 个）、`css/css-color` 里名字带 `opacity` 的（12 个）、`css/css-text-decor/text-shadow`（19 个，去掉了 `onload` 改样式的 1 个）、`css/css-backgrounds` 里名字带 `shadow` 的（25 个，加进上面那一批）（2c-2）：筛法同上，另外去掉 3D（`perspective`、`preserve-3d`、`rotateX/Y`、`matrix3d`……）、单独的 `translate:`/`rotate:`/`scale:` 属性、`filter`、`mask`、动画。
- `css/css-fonts`（2d-1，51 个，`--win` 用 Windows 平台跑；通过的在 `passing-win.txt`）：有 `rel=match|mismatch`、没有 `<script>` 和 2d-2 之后的特性（书写方向、注音、字体变体、`size-adjust` 等）、`@font-face` 或名字带 font-face/font-display/unicode-range/font-matching 的页面和引用的文件；`fonts/` 下的字体是它们引用的。
- `css/css-writing-modes`（98 个）、`css/CSS2/bidi-text`（103 个）、`css/css-text/text-align`（85 个）（2d-2a，测试平台）：有 `rel=match|mismatch`、没有 `<script>`、没有竖排（`writing-mode: vertical-*`、`sideways`）、`text-orientation`、`text-combine`、注音、表单控件、`svg` 等后面步骤特性的页面和它们引用的文件；`css-writing-modes` 只取名字带 bidi、direction、rtl、isolate、embed、override、plaintext、text-align、dir- 的。来源同上（`html/rendering/bidi-rendering` 没有可用的参考测试）。
- `css/css-writing-modes` 的竖排部分（745 个，新增 1073 个文件，含 `reference/`、`support/`）、`css/css-ruby`（118 个，新增 223 个文件）（2d-2b，2026-10-21 加；来源同上，同一个提交）：同样只取有 `rel=match|mismatch`、没有 `<script>`、没有 `reftest-wait`/`reftest-paged` 的页面（参考页和它链接的样式表也要满足），文件名是 `-ref`/`-notref` 或在 `reference/`、`support/` 里的不算测试。
  竖排那批另外要求测试页或它链接的样式表里出现 `writing-mode: vertical-rl|vertical-lr|sideways-rl|sideways-lr`、`text-orientation` 或 `text-combine-upright`，并去掉（测试页、参考页、样式表里任一处出现）`display: flex|grid|table*`、`<table>` 系元素、`svg`、表单控件/`iframe`/`object`/`video` 等、`canvas`、MathML、`contain`、`position: sticky`、`filter`、`mask`/`clip-path`、多栏、`shape-*`、`ruby`，以及用到 Ahem 以外网络字体（`@font-face`、`.woff`/`.ttf`/`.otf`）的页面（浮动保留）；筛前 1139 个 rel=match|mismatch 的测试页，去掉 155 个非竖排、147 个 flex/grid/table、36 个脚本、36 个控件、12 个其他字体、4 个 `contain`、3 个 `canvas`、1 个 `svg`。
  `css-ruby` 只用通用筛法（143 个里去掉 25 个有脚本的），不按特性再筛，失败由引擎这边分类；它引用的 `fonts/mplus-1p-regular.woff`（803 KB）和 `fonts/noto/cjk/NotoSansCJKjp-Regular-subset-halt.otf` 也一起拷了。这两批还没跑过，没有进 `passing.txt`。
- `css/CSS2/support`、`css/CSS2/reference`、`css/reference`、`css/support`、`css/CSS2/positioning/absolute-non-replaced-width-017-ref.xht`：测试引用的图片、参考页、样式表
- `fonts/Ahem.ttf`、`fonts/ahem.css`：Ahem 字体（测试平台里每个字 1em 见方，就是它的样子，所以这一步不加载它）

## 运行

```
qinglan-cli wpt [--root tests/wpt] [--update] <目录>...     # 例如 css/CSS2/floats，写 RESULTS-<目录>.md
qinglan-cli wpt [--root tests/wpt] [--win] [--update] <目录>...   # --win：用 Windows 平台（DirectWrite），结果写 RESULTS-<目录>-win.md、passing-win.txt
qinglan-cli wpt [--root tests/wpt] --check                  # 跑 passing.txt 里的测试，有一个不过就失败
```

- 用测试平台渲染测试页和参考页（800×600，WPT 的默认），逐像素比较；有 `<meta name=fuzzy>` 的按它放宽。
  `rel=match` 的多个参考任一个相同就过，`rel=mismatch` 的必须都不同；参考页自己的 `rel=match` 链也要符合。
- 把 `tests/wpt/` 当作 `http://wpt.test/`，所以 `/fonts/ahem.css` 这样的根路径能用。`.xht` 当 HTML 解析（引擎没有 XML 解析器），
  跑之前去掉 `<![CDATA[` 和 `]]>`、XML 注释，并把自闭合的非空元素补成 `<x></x>`。
- 跳过：有 `<script>` 的（要 JS）、`reftest-wait`、打印（`reftest-paged`）、没有 `rel=match|mismatch` 的（testharness 等）。
- `passing.txt`：现在通过的测试，`cargo test -p qinglan-cli --test wpt` 会跑它，退步就失败。`--update` 在跑完一个目录后重写这个目录的条目。
- `notes.txt`：`路径<TAB>类别<TAB>原因`，人看过失败之后写的说明，优先于自动分类。类别以 `bug` 开头的算 bug 类（要修或逐条写明原因）；`limit (…)` 是引擎或测试平台已知的限制（XHTML 当 HTML 解析、测试平台的字宽、没做的计数制等），`step 2x` 是后面步骤的特性。自动分类按测试页里出现的特性（表格、flex、渐变、
  生成内容……）猜"要第几步的特性"，没有命中的就是 `bug`。
- `RESULTS-*.md`：每个目录的通过率、失败分类和每条失败的一行原因（`qinglan-cli wpt` 写的，不要手改）。
