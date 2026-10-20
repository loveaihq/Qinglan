# 真实网页库

录制命令：`qinglan-cli record <网址> tests/corpus/local/<名字> --width 1280 --height 2400`。录制文件不提交（有版权），
本表记下网址、类别、录制日期。每页都没有登录，不含个人信息；录制不带 Cookie。`zhwiki-rec` 是第 1c-2 步起的性能页（同一个页面，旧录制）。

| 名字 | 网址 | 类别 | 录制日期 | 状态 |
| --- | --- | --- | --- | --- |
| zhwiki-rec | https://zh.wikipedia.org/wiki/%E4%B8%AD%E5%8D%8E%E4%BA%BA%E6%B0%91%E5%85%B1%E5%92%8C%E5%9B%BD | 中文维基（长条目，性能页，旧录制） | 2026-10-15 | HTTP 200 |
| zhwiki-beijing | https://zh.wikipedia.org/wiki/%E5%8C%97%E4%BA%AC%E5%B8%82 | 中文维基（长条目） | 2026-10-16 | HTTP 200 |
| zhwiki-rust | https://zh.wikipedia.org/wiki/Rust | 中文维基 | 2026-10-16 | HTTP 200 |
| zhwiki-apple | https://zh.wikipedia.org/wiki/%E8%8B%B9%E6%9E%9C | 中文维基（短条目） | 2026-10-16 | HTTP 200 |
| enwiki-rust | https://en.wikipedia.org/wiki/Rust_(programming_language) | 英文维基 | 2026-10-16 | HTTP 200 |
| enwiki-hk | https://en.wikipedia.org/wiki/Hong_Kong | 英文维基（长条目） | 2026-10-16 | HTTP 200 |
| enwiki-popul | https://en.wikipedia.org/wiki/List_of_countries_and_dependencies_by_population | 英文维基（大表格） | 2026-10-16 | HTTP 200 |
| enwiki-main | https://en.wikipedia.org/wiki/Main_Page | 英文维基（首页） | 2026-10-16 | HTTP 200 |
| ruanyifeng-weekly | https://www.ruanyifeng.com/blog/2026/10/weekly-issue-415.html | 中文博客 | 2026-10-16 | HTTP 200 |
| ruanyifeng-home | https://www.ruanyifeng.com/blog/ | 中文博客（首页） | 2026-10-16 | HTTP 200 |
| codingnow-blog | https://blog.codingnow.com/ | 中文博客（首页） | 2026-10-16 | HTTP 200 |
| lilianweng | https://lilianweng.github.io/ | 静态博客（Hugo，英文长文） | 2026-10-16 | HTTP 200 |
| chinanews-art | https://www.chinanews.com.cn/gn/2026/10-15/10715019.shtml | 中文新闻正文 | 2026-10-16 | HTTP 200 |
| people-art | http://politics.people.com.cn/n1/2026/0926/c1024-40806006.html | 中文新闻正文 | 2026-10-16 | HTTP 200 |
| xinhua-art | https://www.news.cn/local/20261015/7b544534c7424aea9f7fd7d1e5b4c6da/c.html | 中文新闻正文 | 2026-10-16 | HTTP 200 |
| bbc-zhongwen | https://www.bbc.com/zhongwen/articles/cm2d65npyrdjo/simp | 中文新闻正文 | 2026-10-16 | HTTP 200 |
| mdn-div | https://developer.mozilla.org/en-US/docs/Web/HTML/Element/div | 技术文档（MDN） | 2026-10-16 | HTTP 200 |
| mdn-flex | https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_flexible_box_layout/Basic_concepts_of_flexbox | 技术文档（MDN） | 2026-10-16 | HTTP 200 |
| rust-book-own | https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html | 技术文档（Rust） | 2026-10-16 | HTTP 200 |
| rust-std-vec | https://doc.rust-lang.org/std/vec/struct.Vec.html | 技术文档（Rust，API 页） | 2026-10-16 | HTTP 200 |
| python-tutorial | https://docs.python.org/3/tutorial/introduction.html | 技术文档（Python） | 2026-10-16 | HTTP 200 |
| python-json | https://docs.python.org/3/library/json.html | 技术文档（Python） | 2026-10-16 | HTTP 200 |
| python-zhcn | https://docs.python.org/zh-cn/3/tutorial/introduction.html | 技术文档（Python 中文） | 2026-10-16 | HTTP 200 |
| gh-ripgrep | https://github.com/BurntSushi/ripgrep | GitHub 仓库首页 | 2026-10-16 | HTTP 200 |
| gh-rust | https://github.com/rust-lang/rust | GitHub 仓库首页 | 2026-10-16 | HTTP 200 |
| so-undo-commit | https://stackoverflow.com/questions/927358/how-do-i-undo-the-most-recent-local-commits-in-git | Stack Overflow 问答（被 Cloudflare 拦，403） | 2026-10-16 | HTTP 403 |
| rustusers-topic | https://users.rust-lang.org/t/welcome-to-the-rust-programming-language-users-forum/77411 | 问答页（Discourse 论坛） | 2026-10-16 | HTTP 200 |
| v2ex-topic | https://www.v2ex.com/t/1234 | 问答页（中文论坛主题） | 2026-10-16 | HTTP 200 |
| hn-front | https://news.ycombinator.com/ | Hacker News（表格排版） | 2026-10-16 | HTTP 200 |
| hn-item | https://news.ycombinator.com/item?id=50107379 | Hacker News（评论页） | 2026-10-16 | HTTP 200 |
| zol-gbk | http://www.zol.com.cn/ | GBK 老网站 | 2026-10-16 | HTTP 200 |
| pconline-gbk | http://www.pconline.com.cn/ | GBK 老网站 | 2026-10-16 | HTTP 200 |
| nthu-big5 | http://www.cs.nthu.edu.tw/~king/ | Big5 网站 | 2026-10-16 | HTTP 200 |
| aozora-sjis | https://www.aozora.gr.jp/cards/000148/files/773_14560.html | 日文网站（Shift_JIS） | 2026-10-16 | HTTP 200 |
| jawiki-japan | https://ja.wikipedia.org/wiki/%E6%97%A5%E6%9C%AC | 日文网站（日文维基长条目） | 2026-10-16 | HTTP 200 |
| asahi-home | https://www.asahi.com/ | 日文网站（新闻首页） | 2026-10-16 | HTTP 200 |
| gov-cn-home | https://www.gov.cn/ | 政府网站 | 2026-10-16 | HTTP 200 |
| gov-cn-art | https://www.gov.cn/yaowen/liebiao/202610/content_7083319.htm | 政府网站（正文） | 2026-10-16 | HTTP 200 |
| gov-uk | https://www.gov.uk/ | 政府网站 | 2026-10-16 | HTTP 200 |
| usa-gov | https://www.usa.gov/ | 政府网站 | 2026-10-16 | HTTP 200 |
| kantei-jp | https://www.kantei.go.jp/ | 政府网站（日本） | 2026-10-16 | HTTP 200 |
| matklad | https://matklad.github.io/ | 静态博客（Jekyll） | 2026-10-16 | HTTP 200 |
| jvns | https://jvns.ca/ | 静态博客（Hugo） | 2026-10-16 | HTTP 200 |
| gohugo | https://gohugo.io/ | 静态博客（Hugo 文档） | 2026-10-16 | HTTP 200 |
| jekyllrb | https://jekyllrb.com/ | 静态博客（Jekyll 文档） | 2026-10-16 | HTTP 200 |
| danluu | https://danluu.com/ | 静态博客（纯手写） | 2026-10-16 | HTTP 200 |
| yihui | https://yihui.org/ | 静态博客（Hugo，中英） | 2026-10-16 | HTTP 200 |
| example | https://example.com/ | 极简页面 | 2026-10-16 | HTTP 200 |
| gnu-gpl3 | https://www.gnu.org/licenses/gpl-3.0.html | 长文本（许可证） | 2026-10-16 | HTTP 200 |
| arwiki | https://ar.wikipedia.org/wiki/%D8%A7%D9%84%D9%82%D8%A7%D9%87%D8%B1%D8%A9 | 阿拉伯文维基长条目（从右往左，2d-2a） | 2026-10-20 | HTTP 200 |
| emoji-wiki | https://en.wikipedia.org/wiki/Emoji | 带彩色 emoji 的页面（英文维基 Emoji 条目，2d-2a） | 2026-10-20 | HTTP 200 |
| vert-tategaki | https://tategaki.github.io/ | 竖排（日文，writing-mode: vertical-rl，2d-2b） | 2026-10-21 | HTTP 200 |
