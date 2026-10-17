# Web Platform Tests（reftest 子集）

来源：<https://github.com/web-platform-tests/wpt>，提交 `3bcf9ec39dc0f83e20f5665a988c12c0dff5f1e9`（2026-10-16 取，稀疏浅克隆），
许可证 BSD-3-Clause（见 `LICENSE.md`）。只放用到的目录和它们引用的文件：

- `css/CSS2/floats`、`css/CSS2/floats-clear`、`css/CSS2/generated-content`、`css/css-lists`、`css/CSS2/tables`、`css/CSS2/zindex`、`css/css-tables`：被测的测试（后三个只拷了有 `rel=match|mismatch`、没有 `<script>` 的页面和它们引用的参考页、图片、样式表；testharness 和只能人看的页面没拷，所以比 WPT 里的文件数少很多）
- `css/CSS2/support`、`css/CSS2/reference`、`css/reference`、`css/support`、`css/CSS2/positioning/absolute-non-replaced-width-017-ref.xht`：测试引用的图片、参考页、样式表
- `fonts/Ahem.ttf`、`fonts/ahem.css`：Ahem 字体（测试平台里每个字 1em 见方，就是它的样子，所以这一步不加载它）

## 运行

```
qinglan-cli wpt [--root tests/wpt] [--update] <目录>...     # 例如 css/CSS2/floats，写 RESULTS-<目录>.md
qinglan-cli wpt [--root tests/wpt] --check                  # 跑 passing.txt 里的测试，有一个不过就失败
```

- 用测试平台渲染测试页和参考页（800×600，WPT 的默认），逐像素比较；有 `<meta name=fuzzy>` 的按它放宽。
  `rel=match` 的多个参考任一个相同就过，`rel=mismatch` 的必须都不同；参考页自己的 `rel=match` 链也要符合。
- 把 `tests/wpt/` 当作 `http://wpt.test/`，所以 `/fonts/ahem.css` 这样的根路径能用。`.xht` 当 HTML 解析（引擎没有 XML 解析器），
  跑之前去掉 `<![CDATA[` 和 `]]>`。
- 跳过：有 `<script>` 的（要 JS）、`reftest-wait`、打印（`reftest-paged`）、没有 `rel=match|mismatch` 的（testharness 等）。
- `passing.txt`：现在通过的测试，`cargo test -p qinglan-cli --test wpt` 会跑它，退步就失败。`--update` 在跑完一个目录后重写这个目录的条目。
- `notes.txt`：`路径<TAB>类别<TAB>原因`，人看过失败之后写的说明，优先于自动分类。类别以 `bug` 开头的算 bug 类（要修或逐条写明原因）；`limit (…)` 是引擎或测试平台已知的限制（XHTML 当 HTML 解析、测试平台的字宽、没做的计数制等），`step 2x` 是后面步骤的特性。自动分类按测试页里出现的特性（表格、flex、渐变、
  生成内容……）猜"要第几步的特性"，没有命中的就是 `bug`。
- `RESULTS-*.md`：每个目录的通过率、失败分类和每条失败的一行原因（`qinglan-cli wpt` 写的，不要手改）。
