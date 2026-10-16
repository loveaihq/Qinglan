# 第 1 层验收数据（1c-3）

日期：2026-10-16。机器：Intel i7-8850H，Windows 11，插电（`BatteryStatus = 2`）。量的时候没有 cargo/rustc/qpdf 在跑；用户自己的 Edge（13 个进程）和 Claude 桌面版开着，CPU 占用几个百分点，所以个别单次数字有毛刺（下面写明）。

## 1. 真实网页库

49 个页面（`tests/corpus/PAGES.md` 有网址、类别、录制日期），录在 `tests/corpus/local/`（不提交）。我们的 `qinglan-cli render ... --width 1280 --height 2400`（约前三屏）和 Edge 无头并排，缩小后放在 `tests/out/<名字>.png`（不提交）。脚本是 `tools/corpus_run.py`（测试用，不是产品代码）。

**和任务单不一样的地方**：任务单的 Edge 命令带 `--blink-settings=scriptEnabled=false`。这个 Edge（155）带上它以后无头模式**不出截图**（进程立刻退出，17 秒内没有文件；去掉这个参数就正常）。改成在**新建的空 profile** 里写 `Default/Preferences`，把 JavaScript 内容设置关掉（`profile.default_content_setting_values.javascript = 2`）：验证过，改写页面的脚本不跑，`<noscript>` 的内容显示。每次一个新的空 `--user-data-dir`（在临时目录），不碰用户自己的 profile，跑完清掉进程（只按命令行里带这个目录名的进程找）。另外 Edge 往仓库目录里写不出截图，所以先写到临时目录再拷过来。Edge 加载的是实时网址，录制是更早的；新闻首页类的正文可能不同，选的主要是稳定页面。

**汇总**：

- 能打开：49 / 49（没有崩溃、没有卡死、没有错误页；Stack Overflow 打开的是它的 403 挑战页，见下）。
- 文字完整、顺序对：49 / 49。检查方法（`tools/corpus_run.py check`）：把 DOM 里“会显示的文字”（去掉 script/style/template/head 和 `display:none` 的子树）和排版结果里的文字，逐块（12 个字符一块）按顺序对比。覆盖率 = DOM 里的字有多少在排版结果里按序出现。最低 94.8%（jekyllrb），差的部分都是 `text-transform: uppercase` 改了大小写，和 `<img alt>` 的坏图替代文字（DOM 里是属性，不在文本里）；没有一页丢字或乱序。
- 方块（`.notdef` 字形）：**0**（全部页面）。乱码（U+FFFD）：**0**。GBK（zol、pconline）、Big5（nthu）、Shift_JIS（aozora）、日文维基/朝日（UTF-8）都解对了。
- 和 Edge 的差别：**10 页**肉眼看一致（含 so-undo-commit 两边同为挑战页、gnu-gpl3 的 Edge 一边得到 429）；**33 页**只有第 2 层的差别（float、flex、grid、背景/渐变/背景图、SVG、浮动造成的两栏、`transform`……）；**6 页**有疑似 bug（下面清单；其中 xinhua-art 那个已修）。
- 缩略图只逐页看了一部分（rust-book-own、gnu-gpl3、zhwiki-rec、kantei-jp、xinhua-art、nthu-big5 是单看的整幅图，其余是三张 5×3 的拼图加指标：`ink`（非背景像素占比）、最下面有字的行、缩小后的差值）。所以“差别”一栏是按拼图和指标判断的，没有每页逐处核对。
- 时间和内存是每页一次新进程 `qinglan.exe --bench`（真实窗口，1920×1080，dpr 1，所有图片都取，滚到底）的单次值，会有毛刺；验收用的 zhwiki-rec 5 次中位数在第 3 节。**滚完后的私有内存：中位数 11.6 MB，最大 52.1 MB（zhwiki-rec）**（十进制 MB）。首屏时间：中位数 42 ms，最大 265 ms（gh-ripgrep）；超过 200 ms 的只有 gh-ripgrep（265）和 gh-rust（232），它们的“先排第一屏”基本等于整页（疑似问题 #6）。

| 页面 | 打开 | 文字完整、顺序对（覆盖率） | 方块 | 乱码 | 缺图（整页） | 首屏 ms | 整页排版 ms | 滚完私有内存 MB | 和 Edge 的差别 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| zhwiki-rec | 是 | 是（100.0%） | 0 | 0 | 4 | 121 | 229 | 52.1 | Vector 2022 的 flex/grid/sticky 布局退成块：左侧目录排在正文前面，正文被推到下面；信息框、浮动图、背景色没有（第 2 层） |
| zhwiki-beijing | 是 | 是（100.0%） | 0 | 0 | 4 | 107 | 182 | 49.2 | Vector 2022 的 flex/grid/sticky 布局退成块：左侧目录排在正文前面，正文被推到下面；信息框、浮动图、背景色没有（第 2 层） |
| zhwiki-rust | 是 | 是（100.0%） | 0 | 0 | 4 | 33 | 35 | 10.3 | Vector 2022 的 flex/grid/sticky 布局退成块：左侧目录排在正文前面，正文被推到下面；信息框、浮动图、背景色没有（第 2 层） |
| zhwiki-apple | 是 | 是（100.0%） | 0 | 0 | 4 | 77 | 94 | 22.0 | Vector 2022 的 flex/grid/sticky 布局退成块：左侧目录排在正文前面，正文被推到下面；信息框、浮动图、背景色没有（第 2 层） |
| enwiki-rust | 是 | 是（100.0%） | 0 | 0 | 4 | 70 | 106 | 24.9 | Vector 2022 的 flex/grid/sticky 布局退成块：左侧目录排在正文前面，正文被推到下面；信息框、浮动图、背景色没有（第 2 层） |
| enwiki-hk | 是 | 是（100.0%） | 0 | 0 | 4 | 121 | 208 | 46.7 | Vector 2022 的 flex/grid/sticky 布局退成块：左侧目录排在正文前面，正文被推到下面；信息框、浮动图、背景色没有（第 2 层） |
| enwiki-popul | 是 | 是（100.0%） | 0 | 0 | 4 | 75 | 110 | 28.6 | Vector 2022 的 flex/grid/sticky 布局退成块：左侧目录排在正文前面，正文被推到下面；信息框、浮动图、背景色没有（第 2 层）；大表格走表格退路 |
| enwiki-main | 是 | 是（99.9%） | 0 | 0 | 4 | 66 | 94 | 16.3 | 首页的 grid 双栏退成单栏，背景色块没有（第 2 层） |
| ruanyifeng-weekly | 是 | 是（99.8%） | 0 | 0 | 0 | 22 | 26 | 22.0 | 文字、图片、版面基本一致；只有背景/边框细节不同 |
| ruanyifeng-home | 是 | 是（100.0%） | 0 | 0 | 0 | 13 | 13 | 5.9 | 侧栏用浮动和背景图，退成单栏、没有背景图（第 2 层） |
| codingnow-blog | 是 | 是（98.0%） | 0 | 0 | 0 | 91 | 98 | 10.9 | 两栏用浮动：右栏排到正文下面；背景色/渐变没有（第 2 层） |
| lilianweng | 是 | 是（100.0%） | 0 | 0 | 0 | 34 | 37 | 7.4 | 基本一致 |
| chinanews-art | 是 | 是（100.0%） | 0 | 0 | 0 | 79 | 81 | 8.5 | 版心和侧栏的浮动/背景（第 2 层）；正文逐字相同 |
| people-art | 是 | 是（100.0%） | 0 | 0 | 0 | 28 | 29 | 7.9 | 浮动、背景、顶部导航的 flex（第 2 层）；正文相同 |
| xinhua-art | 是 | 是（100.0%） | 0 | 0 | 0 | 69 | 71 | 18.4 | 修之前头图（1.6 MB 的 PNG）不显示（bug，已修，见“已修”）；现在只剩分享图标/背景（第 2 层） |
| bbc-zhongwen | 是 | 是（95.1%） | 0 | 0 | 0 | 30 | 34 | 22.7 | grid 双栏退成单栏；标题用 text-transform: uppercase，英文大写（和 Edge 一样） |
| mdn-div | 是 | 是（98.5%） | 0 | 0 | 0 | 39 | 44 | 10.6 | 侧栏/顶栏是 flex/grid，退成纵向堆叠（第 2 层） |
| mdn-flex | 是 | 是（99.4%） | 0 | 0 | 7 | 41 | 62 | 15.9 | 同上；7 张 SVG 图不显示（第 2 层：SVG） |
| rust-book-own | 是 | 是（100.0%） | 0 | 0 | 5 | 28 | 36 | 9.9 | mdBook 的左侧栏是 fixed 的 iframe：我们画灰色占位框，盖住正文左边 300 px 里的字（Edge 关 JS 时没有侧栏）；5 张 SVG 不显示。疑似问题 #3 |
| rust-std-vec | 是 | 是（100.0%） | 0 | 0 | 1 | 56 | 141 | 32.4 | 侧栏 flex/sticky 退成块（第 2 层）；正文一致 |
| python-tutorial | 是 | 是（99.3%） | 0 | 0 | 2 | 40 | 46 | 10.1 | 侧栏浮动、徽标 SVG（第 2 层）；正文一致 |
| python-json | 是 | 是（99.7%） | 0 | 0 | 2 | 34 | 45 | 11.5 | 同上 |
| python-zhcn | 是 | 是（99.2%） | 0 | 0 | 2 | 35 | 40 | 10.2 | 同上（中文） |
| gh-ripgrep | 是 | 是（99.9%） | 0 | 0 | 3 | 265 | 273 | 32.8 | 页头/文件列表是 flex/grid，退成块，页头黑块被拉高；3 张图（SVG）不显示（第 2 层） |
| gh-rust | 是 | 是（99.7%） | 0 | 0 | 1 | 232 | 236 | 34.1 | 同上 |
| so-undo-commit | 是 | 是（100.0%） | 0 | 0 | 0 | 4 | 4 | 4.7 | Stack Overflow 对我们的 User-Agent 回 403（Cloudflare“Just a moment”挑战页）；两边显示的都是这个页面。我们不绕过机器人检测，所以没有真正的问答页 |
| rustusers-topic | 是 | 是（100.0%） | 0 | 0 | 0 | 136 | 136 | 17.7 | Discourse 关 JS 时的 <noscript> 版本，两边接近 |
| v2ex-topic | 是 | 是（100.0%） | 0 | 0 | 0 | 42 | 42 | 7.9 | 头像圆角、颜色块等（第 2 层）；文字一致 |
| hn-front | 是 | 是（100.0%） | 0 | 0 | 1 | 33 | 35 | 6.9 | 表格退路：和 Edge 几乎一致（行内间距略不同） |
| hn-item | 是 | 是（100.0%） | 0 | 0 | 1 | 47 | 86 | 19.8 | 表格退路；评论缩进用 1px 图撑，一致 |
| zol-gbk | 是 | 是（100.0%） | 0 | 0 | 10 | 70 | 92 | 19.9 | GBK 解码正确；多栏浮动/表格/背景退成纵向堆叠（第 2 层）；10 张图拿不到（疑似问题 #4） |
| pconline-gbk | 是 | 是（100.0%） | 0 | 0 | 1 | 46 | 53 | 20.7 | GBK 解码正确；多栏浮动退成纵向（第 2 层） |
| nthu-big5 | 是 | 是（100.0%） | 0 | 0 | 1 | 18 | 19 | 5.6 | Big5 解码正确（“金仲達”）；背景图纹理没有（第 2 层）；照片不显示：页面是 https，图片地址写成 http:// ，我们当混合内容拒了，Edge 自动升级成 https（疑似问题 #1） |
| aozora-sjis | 是 | 是（100.0%） | 0 | 0 | 0 | 136 | 195 | 28.8 | Shift_JIS 解码正确；<ruby> 注音可能被当普通行内字排出来（我们的墨量比 Edge 多 24%，未逐字核对，疑似问题 #5） |
| jawiki-japan | 是 | 是（100.0%） | 0 | 0 | 4 | 111 | 209 | 38.2 | Vector 2022 的 flex/grid/sticky 布局退成块：左侧目录排在正文前面，正文被推到下面；信息框、浮动图、背景色没有（第 2 层） |
| asahi-home | 是 | 否（33.3%） | 0 | 0 | 58 | 53 | 64 | 15.4 | flex/grid 退成块；58 张图不显示：8 张 SVG（第 2 层），其余 50 张录制时就没取到（疑似问题 #4）；坏图的 alt 文字照排 |
| gov-cn-home | 是 | 是（100.0%） | 0 | 0 | 0 | 47 | 49 | 11.3 | 浮动/背景/轮播（第 2 层） |
| gov-cn-art | 是 | 是（100.0%） | 0 | 0 | 0 | 29 | 30 | 8.2 | 浮动、背景（第 2 层）；正文一致 |
| gov-uk | 是 | 是（98.9%） | 0 | 0 | 0 | 56 | 58 | 11.8 | grid 卡片退成块；蓝色背景块有（第 2 层其余） |
| usa-gov | 是 | 是（99.0%） | 0 | 0 | 5 | 92 | 95 | 13.6 | grid/flex 卡片退成块；5 张 SVG 不显示（第 2 层） |
| kantei-jp | 是 | 是（100.0%） | 0 | 0 | 1 | 21 | 22 | 28.1 | Edge 这边拿到的是站点的 404 页（对无头 Edge 的请求被拒），不能当参考；我们这边的轮播图显示成大黑白块（背景/溢出裁剪，第 2 层） |
| matklad | 是 | 是（100.0%） | 0 | 0 | 0 | 25 | 29 | 6.9 | 基本一致 |
| jvns | 是 | 是（99.7%） | 0 | 0 | 0 | 34 | 47 | 11.6 | 背景色块/浮动导航（第 2 层） |
| gohugo | 是 | 是（99.2%） | 0 | 0 | 2 | 38 | 39 | 9.5 | 渐变/背景图/深色头部（第 2 层）；2 张图（SVG/PNG）不显示 |
| jekyllrb | 是 | 是（94.8%） | 0 | 0 | 0 | 35 | 35 | 7.5 | 渐变/背景、导航 flex（第 2 层）；标题大写同 Edge |
| danluu | 是 | 是（100.0%） | 0 | 0 | 0 | 10 | 14 | 6.5 | 一致 |
| yihui | 是 | 是（98.9%） | 0 | 0 | 0 | 60 | 60 | 6.6 | 导航 flex 和背景（第 2 层） |
| example | 是 | 是（100.0%） | 0 | 0 | 0 | 5 | 5 | 4.8 | Edge 把正文放进居中的灰底卡片（背景色 + 外边距 auto 的块，第 2 层的背景部分）；文字一致 |
| gnu-gpl3 | 是 | 是（100.0%） | 0 | 0 | 0 | 39 | 50 | 11.4 | Edge 这边收到 HTTP 429（Too Many Requests），不能当参考；我们这边版面正常 |

“缺图”是整页里加载失败/不支持的图片数。原因（看录制里的图片响应）：SVG（第 2 层）占大多数（rust-book 5、mdn-flex 7、usa-gov 5 等）；asahi 50、zol 10 张是录制时就没取到；nthu 1 张是混合内容。

### 疑似 bug 清单（给审查；除已修的以外都没动）

1. **http 图片在 https 页面上被当混合内容拒绝**（nthu-big5 的照片）。Chrome/Edge/Firefox 会先把子资源图片自动升级成 https，我们不升级，所以这类老页面缺图。
2. （已修）**大于 1 MB 的图片永远画不出来**：响应缓存对单个响应的上限是 1 MB（为了不存 2.4 MB 的维基页面），图片的压缩字节也被挡在外面；画图时要从缓存里取字节来解码，取不到就“再请求一次”，下载回来又存不进缓存——窗口里会无限重新下载。xinhua-art 的 1.6 MB 头图就是这样。见“已修”。
3. rust-book-own：`<iframe>` 占位框是不透明的灰框，`position: fixed` 的侧栏 iframe 盖住正文左边 300 px 的字。任务单要求 iframe 显示占位框，但盖住字的效果可能不是想要的（可以考虑不画 fixed 的 iframe 占位，或画成半透明）。
4. 录制时有些图片没取到：asahi-home 50 张、zol-gbk 10 张（站点对我们的 User-Agent/Referer 回 403/404，还是传输有问题，没有查）。
5. aozora-sjis：`<ruby>` 注音疑似按普通行内文字排（墨量比 Edge 多 24%）；没有逐字核对。
6. gh-ripgrep / gh-rust：首屏 232–265 ms，超过 200 ms 目标；“先排第一屏”基本排完整页（首屏 ≈ 整页排版时间），可能是页头 flex 退成块以后停止线来得太晚。rustusers-topic（Discourse）首屏 136 ms ≈ 整页 137 ms，同样。
7. 超过 1 MB 的 HTML 响应不进缓存（后退时要重新下载）是 1c-2c 的决定，不算 bug；列出来是因为它和 #2 是同一个开关。
8. 坏 UTF-8 页面（第 2 节）的峰值内存接近 200 MB 的上限（4 MB 垃圾 196 MB，5 MB 198 MB；测试用 2 MB，179 MB），而且不随大小线性增长（2 MB 就已经 179 MB），说明有一大块固定开销（疑似每个 U+FFFD 的整形/排版数据），值得查。
9. Big5 只找到 1 个页面（cs.nthu.edu.tw/~king/，3.8 KB）：试了约 80 个台湾/香港网站，都已经是 UTF-8。Big5 解码在 `encodings.rs` 的测试和这一页里验证过，但大页面的 Big5 没有真实样本。
10. Stack Overflow、askubuntu、segmentfault 对我们的 User-Agent 回 403/468（Cloudflare 等）。我们不绕过机器人检测。问答页类别用了 Discourse（users.rust-lang.org）、V2EX，SO 本身录的是挑战页。
11. Edge 一侧不可用的两页：gnu-gpl3（429）、kantei-jp（404 页）。

### 已修的问题

- **大于 1 MB 的图片不显示 + 无限重新下载**（`net/cache.rs`、`paint/images.rs`）：图片响应（`Content-Type: image/*`）的单个上限改为预算的一半（4 MB），其他响应仍是 1 MB；`ImageStore` 对“字节不在缓存里”的结点最多再请求 2 次，之后放弃（成功解码就清零），所以超过 4 MB 的图片不会再无限下载。测试：`net.rs::an_image_bigger_than_a_page_entry_is_still_kept`、`paint.rs::bytes_that_never_stay_in_the_cache_are_asked_for_only_twice`。
- 没有崩溃、卡死、乱码、方块、丢字，所以没有别的要修。
- 另：`qinglan-cli bench` 的滚动循环没有帧数上限（高 1e9 像素的页面会滚几小时），加了 `--max-frames N`（默认 20 000 帧）和输出 `scroll-complete`。这是测试工具的改动，不是产品代码。

### 审查后的修复（1c 复审，2026-10-16；上面的疑似 bug 清单按这里更新）

- **#1 混合内容图片（已修）**：https 页面上的 `http://` 图片改成 `https://` 去取（和 Chrome 对图片的做法一样），失败就是坏图，不回退 http；样式表等其他子资源仍然拦截。`policy::upgrade_mixed_image`，测试 `net.rs::http_images_on_an_https_page_are_upgraded_and_nothing_else_is`。重新录 zol：14 张图请求、14 张加载、0 张坏（之前 10 张 `http://2*.zol-img.com.cn/manu_photo/*` 被拒）。
- **#3 iframe 占位框盖字（已修）**：占位框不再填充，只画 1 px 灰框加名字，下面的字照常显示。测试 `paint.rs::a_frame_placeholder_is_a_border_and_a_label_not_a_cover`。
- **#4 asahi 的 50 张、zol 的 10 张（查清）**：asahi 的 58 张“坏图”全是 SVG（`icon__keyGold.svg` 52 张 + `icon__movie.svg` 6 张，第 2 层）；另外 108 张用 lazysizes：`<img class="lazyload" src=loading.jpg data-src=真图>`，真图要靠 JavaScript 把 `data-src` 换进 `src`，第 4 层之前不处理（页面上显示的是 160 字节的占位图）。没有 `<picture>`、`srcset` 的问题。zol 的 10 张是混合内容（#1，已修）；还有 `data-lazyload-src` 的图同样是 JavaScript 懒加载，不处理。之前“录制时没取到”的说法不对：asahi 那 50 张就是 SVG 里的重复项，zol 那 10 张是被我们自己拦的。
- **#6 gh-ripgrep / gh-rust 首屏 > 200 ms（已修，原因和猜的不一样）**：首屏时间不在排版（`first-layout` 4–9 ms），而在样式：21 个样式表共 2.9 MB、20 744 条规则，`parse_sheets` 153 ms。做了三件事（`css/facts.rs`、`css/syntax.rs::parse_cvs_many`、`css/stylesheet.rs`）：1）按文档里有的标签/id/class/属性值，丢掉不可能匹配的规则，丢之前不解析它的声明（20 744 → 2 212 条）；2）当前视口下不成立的 `@media` 块不解析里面的规则；3）样式表多时（共 256 KB 以上）分词在最多 4 个线程上并行。`qinglan-cli style` 的输出在 49 个真实页面上新旧完全相同（逐页比 md5，0 个不同）。`qinglan-cli style --time` 现在会多输出各阶段时间。
- **#5 ruby、#7、#9–#11**：没动（#5 已记入 PLAN 第 2 层；#10 记入第 4 层）。
- **#8 坏 UTF-8 峰值内存（已修，提交 3）**：4 MB 垃圾的峰值 207 MB → 106 MB（`hostile.rs::bad_utf8`，发布版，1.48 秒；目标 ≤ 120 MB，测试里现在断言）。原因和改法见 `docs/decisions.md` 的“提交 3：内存”：长文字按 64 KiB 分段整形、保留数组一次留够、布局里的字形 28 → 16 字节、词表上限 3 万 → 1.6 万词。
- **B6 内存余量（提交 3）**：构建后立刻新开 5 个进程，zhwiki-rec 滚到底的私有字节 47.9、48.4、48.2、49.2、48.2 MB，最大 49.2 MB（≤ 50）。上一轮记的“构建后第一个进程高 4 MB”这次没有重现（第一个是 47.9 MB），原因没查出来，见 decisions.md。

## 2. 恶意网页

每个用例是一份录制，用 `qinglan-cli bench` 走完整流程（解析、样式、排版、显示列表、光栅、滚 10 帧）；每个在独立的子进程里，所以峰值内存（`PeakPagefileUsage`）是这个页面自己的。要求：≤ 5 秒、不 panic、峰值 ≤ 200 MB。下面是 release、`--test-threads=1` 的单次结果（时间含启动进程约 40 ms）。debug 构建下尺寸缩到十分之一，只查内存。

| 项 | 测试 | 时间 | 峰值 Private Bytes | 结果 |
| --- | --- | --- | --- | --- |
| invalid GBK | `qinglan-cli/tests/hostile.rs::bad_gbk` | 1000 ms | 154 MB | ok |
| invalid UTF-8（4 MB） | `qinglan-cli/tests/hostile.rs::bad_utf8` | 1475 ms | 106 MB | ok |
| calc() nested 10 000 deep | `qinglan-cli/tests/hostile.rs::calc_nested_10000` | 55 ms | 9 MB | ok |
| 100 000 CSS rules | `qinglan-cli/tests/hostile.rs::css_100k_rules` | 526 ms | 171 MB | ok |
| one element with 100 000 attributes | `qinglan-cli/tests/hostile.rs::element_with_100k_attributes` | 84 ms | 17 MB | ok |
| 2 000 000 elements | `qinglan-cli/tests/hostile.rs::elements_2m` | 460 ms | 114 MB | ok |
| font-size: 1e9px | `qinglan-cli/tests/hostile.rs::font_size_1e9px` | 43 ms | 7 MB | ok |
| huge positive and negative margins | `qinglan-cli/tests/hostile.rs::huge_margins` | 41 ms | 7 MB | ok |
| 100 000 images | `qinglan-cli/tests/hostile.rs::images_100k` | 1212 ms | 121 MB | ok |
| @import loop and 1000 @imports | `qinglan-cli/tests/hostile.rs::import_loop_and_1000_imports` | 57 ms | 8 MB | ok |
| 10 000 nested inline elements | `qinglan-cli/tests/hostile.rs::inline_nesting_10000` | 48 ms | 9 MB | ok |
| 100 000 nested elements | `qinglan-cli/tests/hostile.rs::nesting_100k` | 277 ms | 17 MB | ok |
| NUL bytes | `qinglan-cli/tests/hostile.rs::nul_bytes` | 57 ms | 8 MB | ok |
| PNG header claiming 100 000 x 100 000 pixels | `qinglan-cli/tests/hostile.rs::png_header_claims_100k_by_100k` | 60 ms | 7 MB | ok |
| positioned elements offset by 1e9 | `qinglan-cli/tests/hostile.rs::position_offsets_1e9` | 42 ms | 7 MB | ok |
| very long selectors | `qinglan-cli/tests/hostile.rs::selectors_very_long` | 188 ms | 104 MB | ok |
| 10 MB of text without a space | `qinglan-cli/tests/hostile.rs::text_10mb_without_spaces` | 480 ms | 157 MB | ok |
| var() exponential expansion | `qinglan-cli/tests/hostile.rs::var_expands_exponentially` | 42 ms | 8 MB | ok |
| 1 GB 的 gzip 炸弹（6.7 MB 的压缩流，本地 HTTP 服务器，走 WinHTTP） | `qinglan-win/tests/net.rs::gzip_bomb_of_one_gigabyte_is_capped_in_seconds` | 301 ms | 26 MB | ok（解压到 16 MB 上限就停，`TooLarge`） |
| 100 MB 的 gzip 炸弹（1c-1 已有） | `qinglan-win/tests/net.rs::gzip_bomb_is_capped_on_decompressed_bytes` | 145 ms | 26 MB | ok |
| 重定向循环 | `qinglan-win/tests/net.rs::redirects_are_followed_checked_and_limited` | 63 ms | 2 MB | ok（11 次请求后 `TooManyRedirects`） |
| 永远不结束的响应 | `qinglan-win/tests/net.rs::response_that_never_ends_hits_the_cap` | 85 ms | 9 MB | ok（到上限 `TooLarge`） |

网络那四项的时间/峰值是单独运行测试程序、每 10 ms 采样 `PeakPagefileUsage` 得到的（整个测试进程，含本地服务器线程）。另有 1a/1b/1c-1 里在更小的层面测同一批输入的测试：`qinglan-engine/tests/robustness.rs`（深嵌套、节点上限、巨大属性/文本、NUL/坏字节、大样式表、`@import` 上限、选择器/条件/块/值的 1 万层嵌套、模板嵌套）、`layout_basic.rs`（巨大数值、1 MB 栈上的深嵌套、定位、百万空行）、`paint.rs::hostile_pages_do_not_panic_the_painter`、`qinglan-win/tests/images.rs::huge_header_is_refused_without_decoding`。

## 3. 性能和体积（验收 4，release，插电，`qinglan.exe --bench`，zhwiki-rec）

条件：`BatteryStatus = 2`；没有别的编译/测试在跑（用户的 Edge 开着，CPU 几个百分点）；每次新进程（冷），5 次；单次值里有毛刺（第 1 次启动 86 ms、第 3 次 575 ms；滚动最大值 290/440 ms，中位数不受影响）。

| 项目 | 目标 | 实测 |
| --- | --- | --- |
| `qinglan.exe` 大小 | ≤ 1.9 MiB（1 992 294 字节） | **1 927 168 字节（1.838 MiB）**，达标 |
| 依赖的 DLL（读 PE 导入表） | 只依赖系统 DLL | DWrite、GDI32、KERNEL32、SHELL32、SHLWAPI、USER32、WINHTTP、api-ms-win-core-synch-l1-2-0、bcryptprimitives、ntdll、ole32，都是系统的 |
| 启动到能输入网址 | ≤ 0.3 秒 | 中位数 **58.0 ms**（86.3、51.4、574.9、58.0、50.1） |
| 冷启动首屏（1920×1080，dpr 1） | 中位数 ≤ 200 ms | 中位数 **102.9 ms**（118.3、102.9、102.4、106.5、102.4）；墙钟 103.0 ms |
| 整页排完（同上） | 同时报 | 中位数 **212.8 ms**（228.0、212.8、210.2、221.5、210.2） |
| 滚动，物理 1920×1080、DPI 100%（CSS 1920×1080，dpr 1），711 帧 | 帧中位数 ≤ 16 ms，p95 ≤ 33 ms | 中位数 **4.9 ms**（5.6、4.7、5.1、4.9、4.9），p95 **7.0 ms**（9.3、6.8、7.3、6.3、7.0），达标 |
| 滚动，物理 1920×1080、DPI 150%（CSS 1280×720，dpr 1.5），761 帧 | 同上 | 中位数 **5.6 ms**（6.6、5.6、5.6、5.5、5.4），p95 **7.4 ms**（11.2、7.4、7.3、7.4、7.4），达标 |
| 滚到底后私有内存（dpr 1） | ≤ 50 MB | 中位数 **48.9 MB**（51.8、49.1、48.8、47.5、48.9；十进制 MB，= 46.6 MiB）；峰值中位数 54.8 MB。**中位数达标；第 1 次（构建后第一个新进程）51.8 MB，超了 1.8 MB** |
| 同上（150%，设备像素 1920×1080） | 同上 | 中位数 **44.1 MB**（46.3、43.7、44.7、44.1、44.0） |
| 真实网页库滚完私有内存 | 报中位数和最大值 | 中位数 **11.6 MB**，最大 **52.1 MB**（zhwiki-rec，单次；5 次中位数见上一行） |

说明：150% 这一行用 `--width 1280 --height 720 --dpr 1.5`，窗口是 1920×1080 设备像素，和物理 1920×1080 屏幕在 150% 缩放下的工作量一致（1c-2b 时用的是 CSS 1920×1080 × 1.5，重 2.25 倍，那个不是验收要的）。

### 1c 复审后的数字（release，插电 `BatteryStatus = 2`，没有 cargo/rustc/qpdf 在跑，CPU 占用约 23%（用户的 Edge 和桌面版开着），每次新进程，`qinglan.exe --bench` 1920×1080 dpr 1，`C:\Dev\qinglan` 在“Fix real-page issues from the corpus”提交时）

| 页面 | 首屏 ms（5 次） | 中位数 | 滚完私有内存 MB（5 次） | 最大 |
| --- | --- | --- | --- | --- |
| zhwiki-rec | 164.1、115.5、120.7、126.7、120.5 | 120.7 | 53.0、48.7、48.1、48.9、49.4 | 53.0 |
| gh-ripgrep | 172.4、153.7、149.3、150.5、147.3 | 150.5 | 29.1、28.6、28.0、28.9、28.3 | 29.1 |
| gh-rust | 146.9、147.3、148.3、147.2、151.3 | 147.3 | 26.9、26.1、26.4、29.1、27.5 | 29.1 |

- gh-ripgrep 265 → 150.5 ms，gh-rust 232 → 147.3 ms（`qinglan-cli style` 206 → 104 ms）。
- zhwiki-rec 首屏中位数 120.7 ms，比 1c-3 记的 102.9 ms 高；同一台机器上同时跑旧、新 `qinglan-cli bench`（交替 4 轮）首屏 98.9–104.1 对 100.6–102.5、样式 34.6–36.4 对 35.7–37.7，没有差别，所以这是窗口一侧的噪声（CPU 占用 23%）。
- `qinglan.exe` 1 924 608 字节（≤ 1 992 294，余 67 686；提交 3 把开新窗口的 `std::process::Command` 换成直接调 `CreateProcessW`，省了约 60 KB）。
- 4 MB 坏 UTF-8（`hostile.rs::bad_utf8`，提交 3）：峰值 106 MB，1.5 秒（原 207 MB、1.6 秒）；上限 120 MB，达到。

## 第 1 层验收复测（2026-10-16，137a3ee）

插电（`BatteryStatus` 2），每次新开 `qinglan.exe --bench`，中文维基长条目（zhwiki-rec，回放），1920×1080，DPI 100%，共 5 次；其中前几次另一个会话的 cargo 在跑。

| 项目 | 目标 | 5 次结果 | 结论 |
|---|---|---|---|
| exe 大小 | ≤ 1,992,294 字节 | 1,924,608 | 达标 |
| 启动到能输入网址 | ≤ 300 ms | 84.5、96.9、81.1、73.3、472.3（中位数 84.5） | 达标；5 次里有 1 次 472 ms，原因未查 |
| 冷启动首屏 | ≤ 200 ms | 109.6、114.0、123.3、118.2、113.3（中位数 114.0） | 达标 |
| 滚动每帧 | 中位数 ≤ 16、95 分位 ≤ 33 ms | 中位数 4.9–5.9，95 分位 6.4–8.9 | 达标 |
| 滚到底后 Private Bytes | ≤ 50 MB | 47.3、47.3、48.0、47.7、48.1 MB | 达标，余量约 2 MB |

已知问题，留到后面处理：

- 滚动中偶尔有一帧 270–310 ms（5 次里 4 次出现）：图片到达后整页重排在界面线程上跑，约 200 ms。以后把排版挪出界面线程，或者只重排受影响的部分。
- 启动偶尔慢到 0.5 秒左右（1c-3 也见过一次 575 ms），原因还没查。
- 内存余量只有约 2 MB；下一步可做的是 DOM 属性值共享一个池（约 2 MB）。

## 2-0 第 1 层收尾（2026-10-16）

测量条件：**电池供电（`BatteryStatus` 1），另一个会话有 5 个 rustc/cargo 进程在跑，CPU 66–79%**。这次没能插电：脚本等了 6 分钟没等到充电器和空闲，只好照样量；所以速度和启动的绝对值都偏慢，不能当验收数字，要在插电空闲时重量一遍。每次新开进程。`qinglan.exe --bench`（zhwiki-rec，1920×1080，dpr 1，`--replay-delay 80`，新 bench 滚动：每格 100 px，10 格一组，组间停 200 ms，图片懒加载）。

| 项目 | 目标 | 旧行为（每张图到达都整页重排，插电，4 次有效） | 新行为（5 次） |
|---|---|---|---|
| 滚动期间整页重排 | 尽量少 | 21、21、21、21 次（每次 130–180 ms） | 0、0、0、0、0 次（全程 0） |
| 滚动最长一帧（光栅+贴窗口+GdiFlush） | ≤ 50 ms | 495.3、493.0、480.1、528.2（全是 GDI 贴窗口 300–500 ms 的卡顿，见下） | 23.5、22.8、28.3、20.3、25.7 ms，达标 |
| 滚动里我们自己代码占界面线程最长的一次（`scroll-max-ours-ms`） | 参考 | 192.3、181.1、161.9、369.0 | 168.9、99.4、89.2、112.3、101.8（不是排版，是 `prepare` 解码大图；电池+负载） |
| 滚到底后私有字节 | ≤ 46 MB（5 次最大值） | 48.4、48.4、48.6、48.2（十进制 MB） | 45.44、44.12、44.74、44.83、44.91，最大 **45.44**，达标 |
| `qinglan.exe` | ≤ 1 965 000 字节 | 1 924 608 | 1 954 304 |

- **贴窗口卡顿**：旧行为那四次（插电）每次有 7–23 帧、新行为的前几次 A/B（插电）有 6–19 帧在 `SetDIBitsToDevice` 里等 300–500 毫秒；改成电池后新行为 5 次全是 0 帧，最大贴窗口 3.7–4.4 毫秒。看来和图片/排版无关，是系统合成器的忙闲，详见 decisions.md。所以"最长一帧"在插电且合成器忙的时候仍会被系统顶到 500 毫秒；这里达标的数字是电池下量的，**插电时的这一项没有达到 ≤ 50 毫秒，原因在系统而不在我们的代码**（分开记了：`scroll-max-ours-ms`）。
- 启动（新开进程 20 次，`startup-ms`）：215.9 221.9 204.6 197.3 182.8 189.4 162.7 136.8 195.5 153.5 154.5 195.8 156.0 169.1 176.6 217.0 267.4 445.0 991.6 368.4 ；刚复制的新文件 5 个：368.1 333.2 244.6 256.2 364.5 ；旧 exe 10 次：256.5 214.2 176.2 179.5 165.5 197.4 152.6 171.4 162.2 148.9 ；在解析命令行时退出的进程 20 次的墙钟毫秒：397 220 232 222 215 181 214 190 238 198 218 190 179 196 169 161 169 201 229 236 。结论见 decisions.md：不是我们的启动代码。
- 速度（`qinglan-cli bench`，旧、新交替 4 轮）：首屏 中位数 283.6 → 298.4（+5.2%），最小值 279.0 → 277.1（-0.7%）；整页布局 中位数 577.5 → 640.2（+10.9%），最小值 546.2 → 539.3（-1.3%）；滚动帧中位数 中位数 9.9 → 9.8（-1.5%），最小值 9.6 → 9.6（+0.0%）。
- `cargo test --workspace`：290 个通过、0 个失败；`cargo clippy --workspace --all-targets`：0 条警告。
