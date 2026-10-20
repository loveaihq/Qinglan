# WPT `css/css-text/text-align`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/css-text/text-align`，用时 7.1 秒。

- 通过 **51 / 85**（60.0%）
- bug 类失败：**0**
- 跳过：无

失败分类：limit (justify details) 6，limit (plaintext paragraphs) 2，limit (text-align-last) 26

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| text-align-center-last-center.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-center-last-start.html, which it must not) |
| text-align-center-last-end.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-center-last-start.html, which it must not) |
| text-align-center-last-justify.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-center-last-start.html, which it must not) |
| text-align-center-last-start.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-center-last-center.html, which it must not) |
| text-align-end-016.html | limit (plaintext paragraphs) | pre[dir=auto] is unicode-bidi: plaintext: each line (after a preserved newline) is a bidi paragraph with its own direction; the engine analyses a block as one paragraph (3750 pixels differ (largest difference 255); first at (29,86): #ffffff, the reference has #ffa500) |
| text-align-end-last-center.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-end-last-start.html, which it must not) |
| text-align-end-last-end.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-end-last-start.html, which it must not) |
| text-align-end-last-justify.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-end-last-start.html, which it must not) |
| text-align-end-last-start.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-end-last-center.html, which it must not) |
| text-align-justify-bidi-control.html | limit (justify details) | text-align: justify with tabs, soft hyphens, bidi controls or forced breaks in the line is not done as css-text-3 7.3 asks (448 pixels differ (largest difference 255); first at (24,16): #000000, the reference has #ffffff) |
| text-align-justify-last-center.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-justify-last-start.html, which it must not) |
| text-align-justify-last-end.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-justify-last-start.html, which it must not) |
| text-align-justify-last-justify.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-justify-last-start.html, which it must not) |
| text-align-justify-last-start.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-justify-last-end.html, which it must not) |
| text-align-justify-shy-001.html | limit (justify details) | text-align: justify with tabs, soft hyphens, bidi controls or forced breaks in the line is not done as css-text-3 7.3 asks (512 pixels differ (largest difference 255); first at (56,40): #000000, the reference has #ffffff) |
| text-align-justify-tabs-001.html | limit (justify details) | text-align: justify with tabs, soft hyphens, bidi controls or forced breaks in the line is not done as css-text-3 7.3 asks (6900 pixels differ (largest difference 255); first at (63,109): #ffffff, the reference has #000000) |
| text-align-justify-tabs-002.html | limit (justify details) | text-align: justify with tabs, soft hyphens, bidi controls or forced breaks in the line is not done as css-text-3 7.3 asks (3750 pixels differ (largest difference 255); first at (63,109): #ffffff, the reference has #000000) |
| text-align-justify-tabs-003.html | limit (justify details) | text-align: justify with tabs, soft hyphens, bidi controls or forced breaks in the line is not done as css-text-3 7.3 asks (10500 pixels differ (largest difference 255); first at (63,109): #ffffff, the reference has #000000) |
| text-align-justify-tabs-004.html | limit (justify details) | text-align: justify with tabs, soft hyphens, bidi controls or forced breaks in the line is not done as css-text-3 7.3 asks (10500 pixels differ (largest difference 255); first at (63,109): #ffffff, the reference has #000000) |
| text-align-last-010.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (9664 pixels differ (largest difference 255); first at (9,65): #000000, the reference has #ffffff) |
| text-align-last-011.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (9664 pixels differ (largest difference 255); first at (9,65): #000000, the reference has #ffffff) |
| text-align-last-center.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (9216 pixels differ (largest difference 255); first at (8,16): #000000, the reference has #ffffff) |
| text-align-last-end.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (9216 pixels differ (largest difference 255); first at (8,16): #000000, the reference has #ffffff) |
| text-align-last-justify-br.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (1024 pixels differ (largest difference 255); first at (24,16): #000000, the reference has #ffffff) |
| text-align-last-justify-rtl.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (2560 pixels differ (largest difference 255); first at (8,16): #ffffff, the reference has #000000) |
| text-align-last-justify.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (2560 pixels differ (largest difference 255); first at (104,16): #000000, the reference has #ffffff) |
| text-align-last-simple.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (9728 pixels differ (largest difference 255); first at (9,41): #000000, the reference has #ffffff) |
| text-align-last-start.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (9216 pixels differ (largest difference 255); first at (8,16): #ffffff, the reference has #000000) |
| text-align-last-wins-001.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (5120 pixels differ (largest difference 255); first at (11,67): #000000, the reference has #ffffff) |
| text-align-start-016.html | limit (plaintext paragraphs) | pre[dir=auto] is unicode-bidi: plaintext: each line (after a preserved newline) is a bidi paragraph with its own direction; the engine analyses a block as one paragraph (3750 pixels differ (largest difference 255); first at (29,86): #ffa500, the reference has #ffffff) |
| text-align-start-last-center.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-start-last-start.html, which it must not) |
| text-align-start-last-end.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-start-last-start.html, which it must not) |
| text-align-start-last-justify.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-start-last-start.html, which it must not) |
| text-align-start-last-start.html | limit (text-align-last) | text-align-last is not implemented (the last line and the lines before a forced break use text-align) (renders like css/css-text/text-align/text-align-start-last-end.html, which it must not) |

## 通过

- text-align-006.html
- text-align-007.html
- text-align-center-last-default.html
- text-align-default-last-default.html
- text-align-end-001.html
- text-align-end-002.html
- text-align-end-003.html
- text-align-end-004.html
- text-align-end-005.html
- text-align-end-006.html
- text-align-end-007.html
- text-align-end-008.html
- text-align-end-009.html
- text-align-end-010.html
- text-align-end-014.html
- text-align-end-015.html
- text-align-end-017.html
- text-align-end-last-default.html
- text-align-justify-001.html
- text-align-justify-002.html
- text-align-justify-003.html
- text-align-justify-004.html
- text-align-justify-005.html
- text-align-justify-006.html
- text-align-justify-last-default.html
- text-align-justifyall-001.html
- text-align-justifyall-002.html
- text-align-justifyall-003.html
- text-align-justifyall-004.html
- text-align-justifyall-005.html
- text-align-justifyall-006.html
- text-align-match-parent-001.html
- text-align-match-parent-01.html
- text-align-match-parent-05.html
- text-align-match-parent-root-logical.html
- text-align-match-parent-root-ltr.html
- text-align-match-parent-root-rtl.html
- text-align-start-001.html
- text-align-start-002.html
- text-align-start-003.html
- text-align-start-004.html
- text-align-start-005.html
- text-align-start-006.html
- text-align-start-007.html
- text-align-start-008.html
- text-align-start-009.html
- text-align-start-010.html
- text-align-start-014.html
- text-align-start-015.html
- text-align-start-017.html
- text-align-start-last-default.html
