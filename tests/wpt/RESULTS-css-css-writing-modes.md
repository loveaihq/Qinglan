# WPT `css/css-writing-modes`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/css-writing-modes`，用时 9.6 秒。

- 通过 **83 / 98**（84.7%）
- bug 类失败：**0**
- 跳过：无

失败分类：limit (plaintext paragraphs) 3，limit (test platform glyph width) 12

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| bidi-isolate-override-001.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (1152 pixels differ (largest difference 255); first at (96,54): #ffffff, the reference has #000000) |
| bidi-isolate-override-002.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (1152 pixels differ (largest difference 255); first at (96,54): #ffffff, the reference has #000000) |
| bidi-isolate-override-003.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (2304 pixels differ (largest difference 255); first at (96,54): #000000, the reference has #ffffff) |
| bidi-isolate-override-004.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (2304 pixels differ (largest difference 255); first at (96,54): #000000, the reference has #ffffff) |
| bidi-override-001.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (1152 pixels differ (largest difference 255); first at (96,54): #ffffff, the reference has #000000) |
| bidi-override-002.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (1152 pixels differ (largest difference 255); first at (96,54): #ffffff, the reference has #000000) |
| bidi-override-003.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (2304 pixels differ (largest difference 255); first at (96,54): #000000, the reference has #ffffff) |
| bidi-override-004.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (2304 pixels differ (largest difference 255); first at (96,54): #000000, the reference has #ffffff) |
| bidi-plaintext-br-001.html | limit (plaintext paragraphs) | unicode-bidi: plaintext starts a new bidi paragraph, with its own base direction, after every forced break (<br> or a preserved newline); the engine analyses a block as one paragraph (decisions.md 2d-2a) (7168 pixels differ (largest difference 255); first at (9,93): #000000, the reference has #ffffff) |
| block-override-001.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (2304 pixels differ (largest difference 255); first at (72,54): #000000, the reference has #ffffff) |
| block-override-004.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (6912 pixels differ (largest difference 255); first at (72,54): #000000, the reference has #ffffff) |
| block-override-isolate-001.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (2304 pixels differ (largest difference 255); first at (72,54): #000000, the reference has #ffffff) |
| block-override-isolate-004.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (6912 pixels differ (largest difference 255); first at (72,54): #000000, the reference has #ffffff) |
| block-plaintext-004.html | limit (plaintext paragraphs) | unicode-bidi: plaintext starts a new bidi paragraph, with its own base direction, after every forced break (<br> or a preserved newline); the engine analyses a block as one paragraph (decisions.md 2d-2a) (8064 pixels differ (largest difference 255); first at (24,102): #000000, the reference has #ffffff) |
| block-plaintext-006.html | limit (plaintext paragraphs) | unicode-bidi: plaintext starts a new bidi paragraph, with its own base direction, after every forced break (<br> or a preserved newline); the engine analyses a block as one paragraph (decisions.md 2d-2a) (39052 pixels differ (largest difference 255); first at (264,110): #000000, the reference has #ffffff) |

## 通过

- bidi-embed-001.html
- bidi-embed-002.html
- bidi-embed-003.html
- bidi-embed-004.html
- bidi-embed-005.html
- bidi-embed-006.html
- bidi-embed-007.html
- bidi-embed-008.html
- bidi-embed-009.html
- bidi-embed-010.html
- bidi-embed-011.html
- bidi-isolate-001.html
- bidi-isolate-002.html
- bidi-isolate-003.html
- bidi-isolate-004.html
- bidi-isolate-005.html
- bidi-isolate-006.html
- bidi-isolate-007.html
- bidi-isolate-008.html
- bidi-isolate-009.html
- bidi-isolate-010.html
- bidi-isolate-011.html
- bidi-isolate-override-005.html
- bidi-isolate-override-006.html
- bidi-isolate-override-007.html
- bidi-isolate-override-008.html
- bidi-isolate-override-009.html
- bidi-isolate-override-010.html
- bidi-isolate-override-011.html
- bidi-isolate-override-012.html
- bidi-normal-001.html
- bidi-normal-002.html
- bidi-normal-003.html
- bidi-normal-004.html
- bidi-normal-005.html
- bidi-normal-006.html
- bidi-normal-007.html
- bidi-normal-008.html
- bidi-normal-009.html
- bidi-normal-010.html
- bidi-normal-011.html
- bidi-override-005.html
- bidi-override-006.html
- bidi-override-007.html
- bidi-override-008.html
- bidi-override-009.html
- bidi-override-010.html
- bidi-override-011.html
- bidi-override-012.html
- bidi-plaintext-001.html
- bidi-plaintext-002.html
- bidi-plaintext-003.html
- bidi-plaintext-004.html
- bidi-plaintext-005.html
- bidi-plaintext-006.html
- bidi-plaintext-007.html
- bidi-plaintext-008.html
- bidi-plaintext-009.html
- bidi-plaintext-010.html
- bidi-plaintext-011.html
- bidi-unset-001.html
- bidi-unset-002.html
- bidi-unset-003.html
- bidi-unset-004.html
- bidi-unset-005.html
- bidi-unset-006.html
- bidi-unset-007.html
- bidi-unset-008.html
- bidi-unset-009.html
- bidi-unset-010.html
- block-embed-001.html
- block-embed-002.html
- block-embed-003.html
- block-flow-direction-004.xht
- block-flow-direction-htb-001.xht
- block-override-002.html
- block-override-003.html
- block-override-isolate-002.html
- block-override-isolate-003.html
- block-plaintext-001.html
- block-plaintext-002.html
- block-plaintext-003.html
- line-box-direction-htb-001.xht
