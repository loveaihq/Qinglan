# WPT `css/css-ruby`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/css-ruby`，用时 1.3 秒。

- 通过 **23 / 118**（19.5%）
- bug 类失败：**0**
- 跳过：no reftest 1

失败分类：limit (ruby as inline-block pairs) 30，not planned 3，step 2d 62

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| block-ruby-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (114902 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #000000) |
| break-within-bases/basic.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (69350 pixels differ (largest difference 255); first at (8,50): #000000, the reference has #ffffff) |
| break-within-bases/no-break-opportunity-at-end.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (no-break-opportunity-at-end) is not modelled (1200 pixels differ (largest difference 255); first at (148,8): #000000, the reference has #ffffff) |
| break-within-bases/nowrap.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (2304 pixels differ (largest difference 255); first at (88,96): #000000, the reference has #ffffff) |
| collapse-trailing-whitespace.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (collapse-trailing-whitespace) is not modelled (1100 pixels differ (largest difference 255); first at (73,50): #ffa500, the reference has #ffffff) |
| empty-ruby-base-container.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (empty-ruby-base-container) is not modelled (12288 pixels differ (largest difference 255); first at (88,8): #ffffff, the reference has #000000) |
| empty-ruby-text-container-float.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (10240 pixels differ (largest difference 255); first at (88,8): #ffffff, the reference has #000000) |
| improperly-contained-annotation-001.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (improperly-contained-annotation-001) is not modelled (2640 pixels differ (largest difference 255); first at (8,8): #000000, the reference has #ffffff) |
| intra-base-white-space-001.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (intra-base-white-space-001) is not modelled (9900 pixels differ (largest difference 255); first at (23,8): #ffffff, the reference has #000000) |
| nested-ruby-pairing-001.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (10592 pixels differ (largest difference 255); first at (170,64): #ffffff, the reference has #000000) |
| pseudo-first-letter.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (pseudo-first-letter) is not modelled (256 pixels differ (largest difference 128); first at (10,16): #000000, the reference has #008000) |
| pseudo-first-line.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (pseudo-first-line) is not modelled (2944 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #ffa500) |
| rb-display-001.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (rb-display-001) is not modelled (4500 pixels differ (largest difference 255); first at (83,8): #ffffff, the reference has #000000) |
| rbc-rtc-basic-001.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (15300 pixels differ (largest difference 255); first at (143,8): #ffffff, the reference has #000000) |
| rt-display-001.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (rt-display-001) is not modelled (4500 pixels differ (largest difference 255); first at (83,8): #ffffff, the reference has #000000) |
| rt-text-indent-001.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (rt-text-indent-001) is not modelled (1400 pixels differ (largest difference 255); first at (100,0): #ffffff, the reference has #000000) |
| ruby-align-001.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (1536 pixels differ (largest difference 255); first at (48,72): #ffffff, the reference has #000000) |
| ruby-align-001a.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (1536 pixels differ (largest difference 255); first at (48,72): #ffffff, the reference has #000000) |
| ruby-align-002.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (4096 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #000000) |
| ruby-align-002a.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (4096 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #000000) |
| ruby-align-space-around.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (4608 pixels differ (largest difference 255); first at (8,24): #000000, the reference has #ffffff) |
| ruby-annotation-pairing-001.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-annotation-pairing-001) is not modelled (3600 pixels differ (largest difference 255); first at (23,60): #ffffff, the reference has #000000) |
| ruby-autohide-001.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-autohide-001) is not modelled (64 pixels differ (largest difference 255); first at (88,23): #000000, the reference has #ffffff) |
| ruby-autohide-003.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-autohide-003) is not modelled (2496 pixels differ (largest difference 255); first at (36,8): #ffffff, the reference has #000000) |
| ruby-autohide-004.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-autohide-004) is not modelled (448 pixels differ (largest difference 255); first at (12,8): #000000, the reference has #ffffff) |
| ruby-base-container-abs.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-base-container-abs) is not modelled (12288 pixels differ (largest difference 255); first at (88,8): #ffffff, the reference has #000000) |
| ruby-base-container-float.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-base-container-float) is not modelled (16384 pixels differ (largest difference 255); first at (8,104): #ffffff, the reference has #000000) |
| ruby-bidi-002.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (9856 pixels differ (largest difference 255); first at (22,16): #ffffff, the reference has #000000) |
| ruby-bidi-003.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-bidi-003) is not modelled (10240 pixels differ (largest difference 255); first at (632,8): #ffffff, the reference has #000000) |
| ruby-box-generation-001.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (59584 pixels differ (largest difference 255); first at (8,32): #ffffff, the reference has #000000) |
| ruby-box-generation-002.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (71168 pixels differ (largest difference 255); first at (8,32): #ffffff, the reference has #000000) |
| ruby-box-generation-003.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (63280 pixels differ (largest difference 255); first at (8,32): #ffffff, the reference has #000000) |
| ruby-box-generation-004.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (64944 pixels differ (largest difference 255); first at (8,32): #ffffff, the reference has #000000) |
| ruby-box-generation-005.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (60080 pixels differ (largest difference 255); first at (8,32): #ffffff, the reference has #000000) |
| ruby-box-model-001.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-box-model-001) is not modelled (99400 pixels differ (largest difference 255); first at (258,80): #ffffff, the reference has #ffc0cb) |
| ruby-float-handling-001.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-float-handling-001) is not modelled (15066 pixels differ (largest difference 255); first at (12,8): #ffffff, the reference has #000000) |
| ruby-inlinize-blocks-001.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (9824 pixels differ (largest difference 255); first at (24,8): #ffffff, the reference has #000000) |
| ruby-inlinize-blocks-002.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-inlinize-blocks-002) is not modelled (33630 pixels differ (largest difference 255); first at (8,8): #000000, the reference has #ffffff) |
| ruby-inlinize-blocks-004.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (9824 pixels differ (largest difference 255); first at (24,8): #ffffff, the reference has #000000) |
| ruby-inlinize-blocks-005.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (13104 pixels differ (largest difference 255); first at (38,8): #ffffff, the reference has #000000) |
| ruby-inlinize-recursive-simple.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-inlinize-recursive-simple) is not modelled (9984 pixels differ (largest difference 255); first at (88,8): #ffffff, the reference has #000000) |
| ruby-intra-level-whitespace-001.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-intra-level-whitespace-001) is not modelled (256 pixels differ (largest difference 255); first at (56,29): #000000, the reference has #ffffff) |
| ruby-intra-level-whitespace-003.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (960 pixels differ (largest difference 255); first at (240,8): #000000, the reference has #ffffff) |
| ruby-intrinsic-isize-001.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-intrinsic-isize-001) is not modelled (8626 pixels differ (largest difference 255); first at (26,44): #ffffff, the reference has #000000) |
| ruby-intrinsic-isize-003.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-intrinsic-isize-003) is not modelled (12432 pixels differ (largest difference 255); first at (66,44): #ffffff, the reference has #000000) |
| ruby-lang-specific-style-001.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (2088 pixels differ (largest difference 255); first at (16,272): #000000, the reference has #ffffff) |
| ruby-line-break-suppression-001.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (2048 pixels differ (largest difference 255); first at (24,8): #ffffff, the reference has #000000) |
| ruby-line-break-suppression-002.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (20568 pixels differ (largest difference 255); first at (169,9): #ffffff, the reference has #000000) |
| ruby-line-break-suppression-003.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (2176 pixels differ (largest difference 255); first at (40,8): #ffffff, the reference has #000000) |
| ruby-line-break-suppression-005.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-line-break-suppression-005) is not modelled (256 pixels differ (largest difference 255); first at (40,8): #000000, the reference has #ffffff) |
| ruby-line-breaking-001.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (5536 pixels differ (largest difference 255); first at (41,25): #ffffff, the reference has #000000) |
| ruby-line-breaking-002.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-line-breaking-002) is not modelled (1024 pixels differ (largest difference 255); first at (25,9): #000000, the reference has #ffffff) |
| ruby-no-transform.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (256 pixels differ (largest difference 255); first at (84,8): #ffffff, the reference has #000000) |
| ruby-overhang-none.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (2688 pixels differ (largest difference 255); first at (8,40): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-001.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (3000 pixels differ (largest difference 255); first at (33,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-002.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (32100 pixels differ (largest difference 255); first at (34,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-003.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (4600 pixels differ (largest difference 255); first at (33,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-004.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (5500 pixels differ (largest difference 255); first at (33,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-005.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (1820 pixels differ (largest difference 255); first at (35,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-006.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (25450 pixels differ (largest difference 255); first at (33,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-007.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (17050 pixels differ (largest difference 255); first at (33,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-008.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (2300 pixels differ (largest difference 255); first at (33,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-009.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (6450 pixels differ (largest difference 255); first at (33,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-010.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (2200 pixels differ (largest difference 255); first at (33,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-011.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (36860 pixels differ (largest difference 255); first at (24,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-012.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (42050 pixels differ (largest difference 255); first at (37,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-013.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (3900 pixels differ (largest difference 255); first at (33,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-014.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (18840 pixels differ (largest difference 255); first at (39,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-015.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (34200 pixels differ (largest difference 255); first at (33,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-016.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (8300 pixels differ (largest difference 255); first at (33,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-017.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (7550 pixels differ (largest difference 255); first at (33,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-018.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (3620 pixels differ (largest difference 255); first at (34,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-019.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (1820 pixels differ (largest difference 255); first at (35,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-021.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (2440 pixels differ (largest difference 255); first at (36,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-align-001.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (1400 pixels differ (largest difference 255); first at (113,48): #ffffff, the reference has #ffa500) |
| ruby-overhang-spaces-align-002.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (1100 pixels differ (largest difference 255); first at (113,48): #ffffff, the reference has #ffa500) |
| ruby-overhang-spaces-align-003.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (2080 pixels differ (largest difference 255); first at (33,48): #000000, the reference has #ffffff) |
| ruby-overhang-spaces-align-004.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (1780 pixels differ (largest difference 255); first at (33,48): #000000, the reference has #ffffff) |
| ruby-overhang-spaces-align-005.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (1400 pixels differ (largest difference 255); first at (93,48): #ffffff, the reference has #ffa500) |
| ruby-overhang-spaces-non-bmp-001.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (2200 pixels differ (largest difference 255); first at (33,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-non-bmp-002.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (2240 pixels differ (largest difference 255); first at (34,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-text-spacing-trim-001.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (32100 pixels differ (largest difference 255); first at (34,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-text-spacing-trim-002.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (34200 pixels differ (largest difference 255); first at (36,48): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-vertical-001.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (5360 pixels differ (largest difference 255); first at (28,32): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-vertical-002.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (12060 pixels differ (largest difference 255); first at (28,32): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-vertical-003.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (9120 pixels differ (largest difference 255); first at (28,32): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-vertical-004.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (88480 pixels differ (largest difference 255); first at (28,32): #ffffff, the reference has #000000) |
| ruby-overhang-spaces-vertical-005.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (11930 pixels differ (largest difference 255); first at (28,32): #ffffff, the reference has #000000) |
| ruby-span-001.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (768 pixels differ (largest difference 255); first at (216,8): #000000, the reference has #ffffff) |
| ruby-tab-in-base-001.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-tab-in-base-001) is not modelled (1588 pixels differ (largest difference 255); first at (76,8): #ffffff, the reference has #000000) |
| ruby-tab-in-base-002.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-tab-in-base-002) is not modelled (3176 pixels differ (largest difference 255); first at (126,8): #ffffff, the reference has #000000) |
| ruby-tab-in-base-003.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-tab-in-base-003) is not modelled (1926 pixels differ (largest difference 255); first at (76,8): #ffffff, the reference has #000000) |
| ruby-text-collapse.html | limit (ruby as inline-block pairs) | ruby is laid out as atomic inline-block pairs (annotation block + base block, decisions.md 2d-2b), so this css-ruby-1 detail (ruby-text-collapse) is not modelled (338 pixels differ (largest difference 255); first at (56,242): #ffffff, the reference has #000000) |
| ruby-whitespace-001.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (20112 pixels differ (largest difference 255); first at (23,112): #000000, the reference has #ffffff) |
| ruby-whitespace-002.html | step 2d | uses complex ruby (rtc, ruby-align, overhang) (1536 pixels differ (largest difference 255); first at (264,8): #ffffff, the reference has #000000) |

## 通过

- abs-in-ruby-base-container.html
- abs-in-ruby-base.html
- abs-in-ruby-container.html
- empty-ruby-text-container-abs.html
- empty-ruby-text-container.html
- interlinear-block-margin-box.html
- ruby-annotation-with-margin.html
- ruby-base-different-size.html
- ruby-bidi-001.html
- ruby-bidi-004.html
- ruby-inlinize-blocks-003.html
- ruby-intrinsic-isize-002.html
- ruby-justification-001.html
- ruby-justification-002.html
- ruby-layout-internal-boxes.html
- ruby-line-break-suppression-004.html
- ruby-overhang-spaces-022.html
- ruby-reflow-001-opaqueruby.html
- ruby-reflow-001-transparentruby.html
- ruby-with-floats-001.html
- ruby-with-floats-002.html
- ruby-with-floats-003.html
- whitespace-ruby-beside-float.html
