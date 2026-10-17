# WPT `css/css-flexbox`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/css-flexbox`，用时 4.5 秒。

- 通过 **647 / 951**（68.0%）
- bug 类失败：**87**
- 跳过：无

失败分类：bug (open) 87，limit (placeholder boxes) 12，limit (test platform) 26，not planned 51，step 2b 4，step 2c 30，step 2d 69，step 2e 25

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| abspos/flex-abspos-align-self-safe-outer-cb-001.tentative.html | step 2c | uses painting effects (23286 pixels differ (largest difference 255); first at (189,42): #800080, the reference has #ffffff) |
| abspos/flex-abspos-align-self-safe-outer-cb-002.tentative.html | step 2c | uses painting effects (25576 pixels differ (largest difference 255); first at (194,42): #800080, the reference has #ffffff) |
| abspos/flex-abspos-staticpos-align-self-safe-001.html | step 2d | uses text features (5852 pixels differ (largest difference 255); first at (31,14): #800080, the reference has #ffffff) |
| abspos/flex-abspos-staticpos-align-self-safe-002.html | step 2d | uses text features (6608 pixels differ (largest difference 255); first at (7,14): #800080, the reference has #ffffff) |
| abspos/flex-abspos-staticpos-fallback-justify-content-001.html | bug (open) | not analysed one by one (a css-flexbox-1 test that still differs from the reference); the families above and in docs/decisions.md 2b-1 list what is known to be missing (144 pixels differ (largest difference 255); first at (38,10): #008080, the reference has #ffff00) |
| align-baseline.html | bug (open) | baseline alignment (css-flexbox-1 8.5, 9.4): items with different font sizes or in vertical lines; the references use offsets worked out with real font metrics, the first baseline of a container is the first item's. Not analysed one by one (8192 pixels differ (largest difference 255); first at (416,144): #ffffff, the reference has #000000) |
| align-content-001.htm | step 2c | uses painting effects (14400 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| align-content-002.htm | step 2c | uses painting effects (14400 pixels differ (largest difference 255); first at (8,116): #ffffff, the reference has #008000) |
| align-content-003.htm | step 2c | uses painting effects (14400 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| align-content-004.htm | step 2c | uses painting effects (11100 pixels differ (largest difference 255); first at (8,85): #ffffff, the reference has #008000) |
| align-content-005.htm | step 2c | uses painting effects (16800 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| align-content-007.htm | step 2c | uses painting effects (40000 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| align-items-001.htm | step 2c | uses painting effects (14400 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| align-items-002.htm | step 2c | uses painting effects (14700 pixels differ (largest difference 255); first at (8,115): #ffffff, the reference has #008000) |
| align-items-003.htm | step 2c | uses painting effects (14700 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| align-items-004.htm | step 2c | uses painting effects (14800 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| align-self-016.html | step 2c | uses painting effects (20000 pixels differ (largest difference 255); first at (108,64): #ff0000, the reference has #008000) |
| alignment/flex-align-baseline-column-vert-lr-rtl-wrap-reverse.html | step 2d | uses text features (5000 pixels differ (largest difference 255); first at (8,98): #ffffff, the reference has #008000) |
| alignment/flex-align-baseline-column-vert-rl-rtl-wrap-reverse.html | step 2d | uses text features (5000 pixels differ (largest difference 255); first at (8,98): #ffffff, the reference has #008000) |
| alignment/flow-start-flow-end-align-properties.html | bug (open) | not analysed one by one (a css-flexbox-1 test that still differs from the reference); the families above and in docs/decisions.md 2b-1 list what is known to be missing (4800 pixels differ (largest difference 255); first at (95,5): #008000, the reference has #ffffff) |
| alignment/flow-start-flow-end-justify-content.html | bug (open) | not analysed one by one (a css-flexbox-1 test that still differs from the reference); the families above and in docs/decisions.md 2b-1 list what is known to be missing (1600 pixels differ (largest difference 255); first at (95,5): #008000, the reference has #ffffff) |
| alignment/flow-start-flow-end-reverse-abspos.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (3200 pixels differ (largest difference 255); first at (95,5): #ffffff, the reference has #008000) |
| alignment/flow-start-flow-end-reverse.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (1600 pixels differ (largest difference 255); first at (95,5): #ffffff, the reference has #008000) |
| alignment/flow-start-flow-end-wrap-reverse.html | bug (open) | not analysed one by one (a css-flexbox-1 test that still differs from the reference); the families above and in docs/decisions.md 2b-1 list what is known to be missing (4800 pixels differ (largest difference 255); first at (95,5): #ffffff, the reference has #008000) |
| aspect-ratio-intrinsic-size-001.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (9061 pixels differ (largest difference 255); first at (18,48): #ffffff, the reference has #008000) |
| aspect-ratio-intrinsic-size-003.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| aspect-ratio-intrinsic-size-006.html | step 2c | uses painting effects (4000 pixels differ (largest difference 255); first at (68,48): #ffffff, the reference has #008000) |
| aspect-ratio-intrinsic-size-008.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (6000 pixels differ (largest difference 255); first at (48,48): #ffffff, the reference has #008000) |
| aspect-ratio-intrinsic-size-009.html | step 2d | uses text features (6000 pixels differ (largest difference 255); first at (48,48): #ffffff, the reference has #008000) |
| aspect-ratio-intrinsic-size-010.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (6000 pixels differ (largest difference 255); first at (48,48): #ffffff, the reference has #008000) |
| aspect-ratio-transferred-max-size.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| auto-margins-002.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (5000 pixels differ (largest difference 255); first at (8,89): #ff0000, the reference has #008000) |
| auto-margins-003.html | step 2d | uses text features (22016 pixels differ (largest difference 255); first at (57,267): #ffffff, the reference has #000000) |
| balance/balance-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (3380 pixels differ (largest difference 255); first at (39,21): #ffffff, the reference has #008000) |
| balance/balance-002.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (2600 pixels differ (largest difference 255); first at (56,21): #000000, the reference has #008000) |
| balance/balance-003.html | not planned | css-flexbox-2 (flex-wrap: balance, min-line-count): not part of css-flexbox-1 (4205 pixels differ (largest difference 255); first at (21,21): #ffa500, the reference has #008000) |
| balance/balance-005.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (9440 pixels differ (largest difference 255); first at (22,21): #ffffff, the reference has #008000) |
| balance/balance-available-size-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (21000 pixels differ (largest difference 255); first at (108,48): #008000, the reference has #ffffff) |
| balance/balance-available-size-002.html | step 2d | uses text features (36000 pixels differ (largest difference 255); first at (248,48): #008000, the reference has #ffffff) |
| balance/balance-complex.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (31200 pixels differ (largest difference 255); first at (18,18): #ffffff, the reference has #008000) |
| balance/balance-flex-grow.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (3400 pixels differ (largest difference 255); first at (29,8): #ffffff, the reference has #008000) |
| balance/balance-line-count-intrinsic-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (6000 pixels differ (largest difference 255); first at (108,64): #008000, the reference has #ffffff) |
| balance/balance-line-count-intrinsic-002.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (24000 pixels differ (largest difference 255); first at (108,64): #008000, the reference has #ffffff) |
| balance/balance-line-count-intrinsic-003.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10000 pixels differ (largest difference 255); first at (108,64): #008000, the reference has #ffffff) |
| balance/balance-line-count-intrinsic-004.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10000 pixels differ (largest difference 255); first at (108,64): #008000, the reference has #ffffff) |
| balance/balance-line-count-intrinsic-005.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (8400 pixels differ (largest difference 255); first at (48,64): #ffffff, the reference has #008000) |
| balance/balance-line-count-intrinsic-006.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10000 pixels differ (largest difference 255); first at (48,64): #ffffff, the reference has #008000) |
| balance/balance-line-count-intrinsic-007.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10000 pixels differ (largest difference 255); first at (48,64): #ffffff, the reference has #008000) |
| balance/balance-min-line-count-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10000 pixels differ (largest difference 255); first at (108,64): #008000, the reference has #ffffff) |
| balance/balance-min-line-count-002.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10000 pixels differ (largest difference 255); first at (108,64): #008000, the reference has #ffffff) |
| balance/balance-min-line-count-003.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (4800 pixels differ (largest difference 255); first at (128,48): #008000, the reference has #ffffff) |
| balance/balance-min-line-count-004.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (5600 pixels differ (largest difference 255); first at (88,48): #008000, the reference has #ffffff) |
| balance/balance-min-line-count-005.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (5600 pixels differ (largest difference 255); first at (88,48): #008000, the reference has #ffffff) |
| balance/balance-min-line-count-006.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (1980 pixels differ (largest difference 255); first at (31,71): #ffffff, the reference has #008000) |
| balance/balance-negative-margin-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (5000 pixels differ (largest difference 255); first at (8,114): #ff0000, the reference has #008000) |
| balance/balance-percentage-size-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (1080 pixels differ (largest difference 255); first at (11,127): #000000, the reference has #ffffff) |
| balance/balance-start-bias-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (13705 pixels differ (largest difference 255); first at (21,21): #ffffff, the reference has #008000) |
| balance/balance-start-bias-002.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (13630 pixels differ (largest difference 255); first at (21,21): #ffffff, the reference has #008000) |
| balance/balance-start-bias-003.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (5835 pixels differ (largest difference 255); first at (27,21): #ffffff, the reference has #008000) |
| balance/balance-zero-size.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (5000 pixels differ (largest difference 255); first at (8,114): #ffffff, the reference has #008000) |
| baseline-outside-flex-item.html | bug (open) | baseline alignment (css-flexbox-1 8.5, 9.4): items with different font sizes or in vertical lines; the references use offsets worked out with real font metrics, the first baseline of a container is the first item's. Not analysed one by one (11300 pixels differ (largest difference 255); first at (8,164): #ff0000, the reference has #ffffff) |
| baseline-synthesis-001.html | step 2d | uses text features (8200 pixels differ (largest difference 255); first at (72,64): #ffffff, the reference has #008000) |
| baseline-synthesis-002.html | step 2d | uses text features (41700 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| baseline-synthesis-003.html | step 2d | uses text features (66560 pixels differ (largest difference 255); first at (108,64): #ff0000, the reference has #ffffff) |
| baseline-synthesis-004.html | step 2d | uses text features (66560 pixels differ (largest difference 255); first at (108,64): #ff0000, the reference has #ffffff) |
| baseline-synthesis-vert-lr-line-under.html | step 2d | uses text features (36700 pixels differ (largest difference 255); first at (108,64): #008000, the reference has #ffffff) |
| canvas-contain-size.html | limit (placeholder boxes) | canvas, video, iframe and object are grey placeholder boxes in layer 2 (no content, no intrinsic size), and fieldset has no special layout; the reference needs the real thing (1916 pixels differ (largest difference 118); first at (8,48): #767676, the reference has #008000) |
| content-height-with-scrollbars.html | step 2e | uses svg, scrolling, sticky or form controls (69200 pixels differ (largest difference 82); first at (400,296): #ffffff, the reference has #add8e6) |
| cross-axis-scrollbar.html | step 2d | uses text features (29500 pixels differ (largest difference 255); first at (150,16): #008000, the reference has #ffffff) |
| css-box-justify-content.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (9140 pixels differ (largest difference 248); first at (0,64): #2977f8, the reference has #ffffff) |
| css-flexbox-row-reverse-wrap-reverse.html | step 2d | uses text features (8760 pixels differ (largest difference 255); first at (42,80): #000000, the reference has #ffffff) |
| css-flexbox-row-reverse-wrap.html | step 2d | uses text features (8760 pixels differ (largest difference 255); first at (42,80): #000000, the reference has #ffffff) |
| css-flexbox-row-reverse.html | step 2d | uses text features (8760 pixels differ (largest difference 255); first at (42,80): #000000, the reference has #ffffff) |
| css-flexbox-row-wrap-reverse.html | step 2d | uses text features (7320 pixels differ (largest difference 255); first at (42,80): #000000, the reference has #ffffff) |
| css-flexbox-row-wrap.html | step 2d | uses text features (7320 pixels differ (largest difference 255); first at (42,80): #000000, the reference has #ffffff) |
| css-flexbox-row.html | step 2d | uses text features (6060 pixels differ (largest difference 255); first at (42,80): #000000, the reference has #ffffff) |
| css-flexbox-test1.html | step 2d | uses text features (6144 pixels differ (largest difference 255); first at (56,112): #008000, the reference has #ffffff) |
| fieldset-as-item-overflow.html | limit (placeholder boxes) | canvas, video, iframe and object are grey placeholder boxes in layer 2 (no content, no intrinsic size), and fieldset has no special layout; the reference needs the real thing (10000 pixels differ (largest difference 255); first at (11,111): #000000, the reference has #008000) |
| fieldset-baseline-alignment.html | bug (open) | baseline alignment (css-flexbox-1 8.5, 9.4): items with different font sizes or in vertical lines; the references use offsets worked out with real font metrics, the first baseline of a container is the first item's. Not analysed one by one (4096 pixels differ (largest difference 255); first at (8,16): #000000, the reference has #ffffff) |
| fixed-table-layout-with-percentage-width-in-flex-item.html | bug (open) | a table as a flex item: its min-content / stretched cross size follows the plain table algorithm with the forced size; Edge sizes the table wrapper box (css-tables-3) differently when it is a flex item. Not analysed one by one (53748 pixels differ (largest difference 255); first at (11,9): #ff0000, the reference has #0000ff) |
| flex-abspos-inset-nested-001.html | bug (open) | out-of-flow descendants of a nested flex container: their containing block / static position when an inset or `order` is involved is not matched. Not analysed one by one (39800 pixels differ (largest difference 255); first at (9,48): #ffffff, the reference has #008000) |
| flex-abspos-inset-nested-002.html | bug (open) | out-of-flow descendants of a nested flex container: their containing block / static position when an inset or `order` is involved is not matched. Not analysed one by one (89700 pixels differ (largest difference 255); first at (9,48): #ffffff, the reference has #008000) |
| flex-aspect-ratio-content-box-padding-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (80000 pixels differ (largest difference 255); first at (8,208): #ffffff, the reference has #008000) |
| flex-aspect-ratio-cross-size-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (199 pixels differ (largest difference 255); first at (8,265): #ffffff, the reference has #008000) |
| flex-aspect-ratio-cross-size-002.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10000 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| flex-aspect-ratio-img-column-004.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (10000 pixels differ (largest difference 255); first at (8,114): #ffffff, the reference has #008000) |
| flex-aspect-ratio-img-column-008.html | step 2d | uses text features (5200 pixels differ (largest difference 255); first at (68,64): #ff0000, the reference has #008000) |
| flex-aspect-ratio-img-column-012.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (5000 pixels differ (largest difference 255); first at (8,148): #008000, the reference has #ffffff) |
| flex-aspect-ratio-img-column-013.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| flex-aspect-ratio-img-column-014.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (10000 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| flex-aspect-ratio-img-column-015.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (10000 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| flex-aspect-ratio-img-column-018.html | step 2e | uses svg, scrolling, sticky or form controls (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| flex-aspect-ratio-img-row-006.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10000 pixels differ (largest difference 255); first at (108,48): #008000, the reference has #ffffff) |
| flex-aspect-ratio-img-row-007.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (12500 pixels differ (largest difference 255); first at (108,48): #008000, the reference has #ffffff) |
| flex-aspect-ratio-img-row-008.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| flex-aspect-ratio-img-row-009.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| flex-aspect-ratio-img-row-010.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (10000 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| flex-aspect-ratio-img-row-011.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (10000 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| flex-aspect-ratio-img-row-015.html | step 2e | uses svg, scrolling, sticky or form controls (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| flex-aspect-ratio-img-row-017.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (9900 pixels differ (largest difference 255); first at (18,64): #ff0000, the reference has #008000) |
| flex-aspect-ratio-intrinsic-padding-001.html | step 2e | uses svg, scrolling, sticky or form controls (33600 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #008000) |
| flex-child-percent-basis-resize-1.html | step 2e | uses svg, scrolling, sticky or form controls (3520 pixels differ (largest difference 255); first at (8,108): #ffffff, the reference has #000000) |
| flex-container-line-if-empty-vertical-writing-mode.html | step 2c | uses painting effects (76400 pixels differ (largest difference 255); first at (28,64): #008000, the reference has #ffffff) |
| flex-container-max-content-001.html | step 2b | uses grid (step 2b-2) (62720 pixels differ (largest difference 255); first at (58,8): #ffffff, the reference has #0000ff) |
| flex-container-min-content-001.html | step 2b | uses grid (step 2b-2) (64299 pixels differ (largest difference 255); first at (38,8): #ffffff, the reference has #0000ff) |
| flex-grow-001.xht | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (14400 pixels differ (largest difference 255); first at (9,9): #008000, the reference has #ffffff) |
| flex-grow-009.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (10000 pixels differ (largest difference 255); first at (8,48): #ff0000, the reference has #008000) |
| flex-item-max-height-min-content.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (10000 pixels differ (largest difference 255); first at (8,148): #008000, the reference has #ffffff) |
| flex-item-transferred-sizes-padding-border-sizing.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (5000 pixels differ (largest difference 255); first at (58,48): #ffffff, the reference has #008000) |
| flex-item-transferred-sizes-padding-content-sizing.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (9000 pixels differ (largest difference 255); first at (18,48): #ffffff, the reference has #008000) |
| flex-lines/multi-line-wrap-reverse-column-reverse.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (149060 pixels differ (largest difference 255); first at (533,10): #00ffff, the reference has #ffffff) |
| flex-lines/multi-line-wrap-with-column-reverse.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (149060 pixels differ (largest difference 255); first at (533,10): #000000, the reference has #ffffff) |
| flex-minimum-width-flex-items-013.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (34600 pixels differ (largest difference 255); first at (108,114): #008000, the reference has #ffffff) |
| flex-minimum-width-flex-items-015.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (10000 pixels differ (largest difference 255); first at (108,64): #008000, the reference has #ffffff) |
| flex-minimum-width-flex-items-016.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (30000 pixels differ (largest difference 255); first at (108,48): #008000, the reference has #ffffff) |
| flex-svg-no-intrinsic-column-001.html | step 2e | uses svg, scrolling, sticky or form controls (22500 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| flexbox-align-self-baseline-horiz-001a.xhtml | bug (open) | baseline alignment (css-flexbox-1 8.5, 9.4): items with different font sizes or in vertical lines; the references use offsets worked out with real font metrics, the first baseline of a container is the first item's. Not analysed one by one (68690 pixels differ (largest difference 255); first at (9,9): #ffffff, the reference has #000000) |
| flexbox-align-self-baseline-horiz-001b.xhtml | bug (open) | baseline alignment (css-flexbox-1 8.5, 9.4): items with different font sizes or in vertical lines; the references use offsets worked out with real font metrics, the first baseline of a container is the first item's. Not analysed one by one (70520 pixels differ (largest difference 255); first at (9,9): #ffffff, the reference has #000000) |
| flexbox-align-self-baseline-horiz-005.xhtml | bug (open) | baseline alignment (css-flexbox-1 8.5, 9.4): items with different font sizes or in vertical lines; the references use offsets worked out with real font metrics, the first baseline of a container is the first item's. Not analysed one by one (3154 pixels differ (largest difference 255); first at (38,15): #ffffff, the reference has #000000) |
| flexbox-align-self-baseline-horiz-006.xhtml | step 2d | uses text features (9508 pixels differ (largest difference 255); first at (9,9): #ffffff, the reference has #000000) |
| flexbox-align-self-baseline-horiz-008.xhtml | step 2d | uses text features (9940 pixels differ (largest difference 255); first at (9,9): #000000, the reference has #ffffff) |
| flexbox-align-self-horiz-001-block.xhtml | bug (open) | not analysed one by one (a css-flexbox-1 test that still differs from the reference); the families above and in docs/decisions.md 2b-1 list what is known to be missing (7500 pixels differ (largest difference 255); first at (389,9): #000000, the reference has #ffc0cb) |
| flexbox-align-self-horiz-001-table.xhtml | bug (open) | not analysed one by one (a css-flexbox-1 test that still differs from the reference); the families above and in docs/decisions.md 2b-1 list what is known to be missing (12700 pixels differ (largest difference 255); first at (389,9): #000000, the reference has #ffc0cb) |
| flexbox-align-self-horiz-002.xhtml | step 2c | uses painting effects (31926 pixels differ (largest difference 255); first at (141,10): #ffffff, the reference has #000000) |
| flexbox-align-self-horiz-003.xhtml | bug (open) | not analysed one by one (a css-flexbox-1 test that still differs from the reference); the families above and in docs/decisions.md 2b-1 list what is known to be missing (7540 pixels differ (largest difference 255); first at (132,0): #000000, the reference has #ffffff) |
| flexbox-align-self-horiz-004.xhtml | step 2c | uses painting effects (18626 pixels differ (largest difference 255); first at (208,0): #000000, the reference has #ffffff) |
| flexbox-align-self-vert-002.xhtml | step 2c | uses painting effects (44868 pixels differ (largest difference 255); first at (417,10): #000000, the reference has #ffffff) |
| flexbox-align-self-vert-rtl-001.xhtml | step 2d | uses text features (14400 pixels differ (largest difference 255); first at (119,399): #ffffff, the reference has #000000) |
| flexbox-align-self-vert-rtl-002.xhtml | step 2c | uses painting effects (30208 pixels differ (largest difference 255); first at (277,10): #ffffff, the reference has #000000) |
| flexbox-align-self-vert-rtl-003.xhtml | step 2d | uses text features (8072 pixels differ (largest difference 255); first at (61,101): #000000, the reference has #ffffff) |
| flexbox-align-self-vert-rtl-004.xhtml | step 2c | uses painting effects (16038 pixels differ (largest difference 255); first at (109,10): #ffffff, the reference has #000000) |
| flexbox-align-self-vert-rtl-005.xhtml | step 2c | uses painting effects (30208 pixels differ (largest difference 255); first at (277,10): #ffffff, the reference has #000000) |
| flexbox-baseline-multi-item-horiz-001a.html | bug (open) | baseline alignment (css-flexbox-1 8.5, 9.4): items with different font sizes or in vertical lines; the references use offsets worked out with real font metrics, the first baseline of a container is the first item's. Not analysed one by one (400 pixels differ (largest difference 255); first at (84,8): #ffffff, the reference has #000000) |
| flexbox-baseline-multi-item-horiz-001b.html | bug (open) | baseline alignment (css-flexbox-1 8.5, 9.4): items with different font sizes or in vertical lines; the references use offsets worked out with real font metrics, the first baseline of a container is the first item's. Not analysed one by one (400 pixels differ (largest difference 255); first at (84,8): #ffffff, the reference has #000000) |
| flexbox-baseline-multi-line-horiz-001.html | bug (open) | baseline alignment (css-flexbox-1 8.5, 9.4): items with different font sizes or in vertical lines; the references use offsets worked out with real font metrics, the first baseline of a container is the first item's. Not analysed one by one (2608 pixels differ (largest difference 255); first at (92,8): #ffffff, the reference has #000000) |
| flexbox-baseline-multi-line-horiz-002.html | bug (open) | baseline alignment (css-flexbox-1 8.5, 9.4): items with different font sizes or in vertical lines; the references use offsets worked out with real font metrics, the first baseline of a container is the first item's. Not analysed one by one (2248 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #000000) |
| flexbox-baseline-multi-line-horiz-004.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (5176 pixels differ (largest difference 255); first at (96,8): #ffffff, the reference has #d3d3d3) |
| flexbox-baseline-multi-line-vert-001.html | bug (open) | baseline alignment (css-flexbox-1 8.5, 9.4): items with different font sizes or in vertical lines; the references use offsets worked out with real font metrics, the first baseline of a container is the first item's. Not analysed one by one (3888 pixels differ (largest difference 255); first at (8,8): #000000, the reference has #ffffff) |
| flexbox-baseline-multi-line-vert-002.html | bug (open) | baseline alignment (css-flexbox-1 8.5, 9.4): items with different font sizes or in vertical lines; the references use offsets worked out with real font metrics, the first baseline of a container is the first item's. Not analysed one by one (3800 pixels differ (largest difference 255); first at (8,8): #000000, the reference has #ffffff) |
| flexbox-basic-block-horiz-001v.xhtml | step 2d | uses text features (1725 pixels differ (largest difference 255); first at (179,417): #ffff00, the reference has #800080) |
| flexbox-basic-block-vert-001v.xhtml | step 2d | uses text features (1035 pixels differ (largest difference 255); first at (217,179): #ffff00, the reference has #800080) |
| flexbox-basic-canvas-vert-001.xhtml | limit (placeholder boxes) | canvas, video, iframe and object are grey placeholder boxes in layer 2 (no content, no intrinsic size), and fieldset has no special layout; the reference needs the real thing (9672 pixels differ (largest difference 255); first at (56,8): #000000, the reference has #90ee90) |
| flexbox-basic-canvas-vert-001v.xhtml | step 2d | uses text features (9672 pixels differ (largest difference 255); first at (56,8): #000000, the reference has #90ee90) |
| flexbox-basic-fieldset-vert-001.xhtml | limit (placeholder boxes) | canvas, video, iframe and object are grey placeholder boxes in layer 2 (no content, no intrinsic size), and fieldset has no special layout; the reference needs the real thing (6472 pixels differ (largest difference 255); first at (56,8): #000000, the reference has #90ee90) |
| flexbox-basic-iframe-vert-001.xhtml | limit (placeholder boxes) | canvas, video, iframe and object are grey placeholder boxes in layer 2 (no content, no intrinsic size), and fieldset has no special layout; the reference needs the real thing (9672 pixels differ (largest difference 255); first at (56,8): #000000, the reference has #90ee90) |
| flexbox-basic-img-vert-001.xhtml | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (6472 pixels differ (largest difference 255); first at (56,8): #000000, the reference has #90ee90) |
| flexbox-basic-textarea-horiz-001.xhtml | step 2c | uses painting effects (5145 pixels differ (largest difference 255); first at (8,53): #ffffff, the reference has #90ee90) |
| flexbox-basic-textarea-vert-001.xhtml | step 2c | uses painting effects (7076 pixels differ (largest difference 255); first at (28,8): #ffffff, the reference has #90ee90) |
| flexbox-basic-video-vert-001.xhtml | limit (placeholder boxes) | canvas, video, iframe and object are grey placeholder boxes in layer 2 (no content, no intrinsic size), and fieldset has no special layout; the reference needs the real thing (9672 pixels differ (largest difference 255); first at (56,8): #000000, the reference has #90ee90) |
| flexbox-collapsed-item-baseline-001.html | bug (open) | baseline alignment (css-flexbox-1 8.5, 9.4): items with different font sizes or in vertical lines; the references use offsets worked out with real font metrics, the first baseline of a container is the first item's. Not analysed one by one (360 pixels differ (largest difference 255); first at (14,59): #ffff00, the reference has #000000) |
| flexbox-collapsed-item-horiz-002.html | bug (open) | visibility: collapse on a flex item is the simplified version of 4.4 (the item takes no main size but keeps its laid-out cross size); the second layout round with a strut per line is not done (1470 pixels differ (largest difference 255); first at (14,153): #808000, the reference has #ffff00) |
| flexbox-collapsed-item-horiz-003.html | bug (open) | visibility: collapse on a flex item is the simplified version of 4.4 (the item takes no main size but keeps its laid-out cross size); the second layout round with a strut per line is not done (400 pixels differ (largest difference 255); first at (14,34): #808000, the reference has #ffff00) |
| flexbox-column-row-gap-002.html | step 2d | uses text features (9760 pixels differ (largest difference 255); first at (29,9): #ffffff, the reference has #0000ff) |
| flexbox-column-row-gap-004.html | step 2d | uses text features (14516 pixels differ (largest difference 255); first at (112,268): #000000, the reference has #ffffff) |
| flexbox-definite-cross-size-constrained-percentage.html | bug (open) | definite sizes (css-flexbox-1 9.8) for percentages inside items: a stretched item or an item of an auto-height container is not always definite for its children; and percentage widths wait for the shrunk size. Not analysed one by one (875 pixels differ (largest difference 255); first at (33,8): #008000, the reference has #ffffff) |
| flexbox-definite-sizes-002.html | bug (open) | definite sizes (css-flexbox-1 9.8) for percentages inside items: a stretched item or an item of an auto-height container is not always definite for its children; and percentage widths wait for the shrunk size. Not analysed one by one (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| flexbox-definite-sizes-003.html | bug (open) | definite sizes (css-flexbox-1 9.8) for percentages inside items: a stretched item or an item of an auto-height container is not always definite for its children; and percentage widths wait for the shrunk size. Not analysed one by one (10816 pixels differ (largest difference 255); first at (8,16): #ffffff, the reference has #000000) |
| flexbox-definite-sizes-004.html | bug (open) | definite sizes (css-flexbox-1 9.8) for percentages inside items: a stretched item or an item of an auto-height container is not always definite for its children; and percentage widths wait for the shrunk size. Not analysed one by one (11872 pixels differ (largest difference 255); first at (8,8): #008000, the reference has #ffffff) |
| flexbox-definite-sizes-005.html | step 2e | uses svg, scrolling, sticky or form controls (188160 pixels differ (largest difference 255); first at (8,80): #ffffff, the reference has #00ff00) |
| flexbox-flex-basis-content-001a.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (2776 pixels differ (largest difference 255); first at (69,102): #ffffff, the reference has #008080) |
| flexbox-flex-basis-content-001b.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (2776 pixels differ (largest difference 255); first at (69,102): #ffffff, the reference has #008080) |
| flexbox-flex-wrap-flexing-003.html | step 2d | uses text features (2500 pixels differ (largest difference 255); first at (58,48): #ffffff, the reference has #008000) |
| flexbox-iframe-intrinsic-size-001.html | limit (placeholder boxes) | canvas, video, iframe and object are grey placeholder boxes in layer 2 (no content, no intrinsic size), and fieldset has no special layout; the reference needs the real thing (6232 pixels differ (largest difference 175); first at (8,64): #767676, the reference has #0000ff) |
| flexbox-items-as-stacking-contexts-001.xhtml | bug (open) | not analysed one by one (a css-flexbox-1 test that still differs from the reference); the families above and in docs/decisions.md 2b-1 list what is known to be missing (125 pixels differ (largest difference 255); first at (13,44): #808080, the reference has #0000ff) |
| flexbox-justify-content-horiz-006.xhtml | step 2d | uses text features (24800 pixels differ (largest difference 203); first at (8,8): #ffffff, the reference has #90ee90) |
| flexbox-justify-content-wmvert-001.xhtml | step 2d | uses text features (392960 pixels differ (largest difference 255); first at (8,8): #90ee90, the reference has #ffffff) |
| flexbox-mbp-horiz-002v.xhtml | step 2c | uses painting effects (2425 pixels differ (largest difference 255); first at (171,408): #ffff00, the reference has #000000) |
| flexbox-min-height-auto-002a.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (2440 pixels differ (largest difference 255); first at (10,40): #00006c, the reference has #800080) |
| flexbox-min-height-auto-002c.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (2440 pixels differ (largest difference 255); first at (10,40): #00006c, the reference has #800080) |
| flexbox-min-width-auto-002a.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (2440 pixels differ (largest difference 255); first at (42,8): #800080, the reference has #ffffff) |
| flexbox-min-width-auto-002b.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (10288 pixels differ (largest difference 255); first at (42,8): #800080, the reference has #ffffff) |
| flexbox-min-width-auto-002c.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (2440 pixels differ (largest difference 255); first at (42,8): #800080, the reference has #ffffff) |
| flexbox-min-width-auto-005.html | bug (open) | not analysed one by one (a css-flexbox-1 test that still differs from the reference); the families above and in docs/decisions.md 2b-1 list what is known to be missing (6000 pixels differ (largest difference 255); first at (48,64): #ff0000, the reference has #008000) |
| flexbox-min-width-auto-006.html | step 2e | uses svg, scrolling, sticky or form controls (14000 pixels differ (largest difference 255); first at (9,65): #ffffff, the reference has #008000) |
| flexbox-overflow-horiz-001.html | step 2e | uses svg, scrolling, sticky or form controls (4080 pixels differ (largest difference 128); first at (61,8): #008080, the reference has #800080) |
| flexbox-paint-ordering-002.xhtml | bug (open) | not analysed one by one (a css-flexbox-1 test that still differs from the reference); the families above and in docs/decisions.md 2b-1 list what is known to be missing (244 pixels differ (largest difference 238); first at (88,88): #800080, the reference has #add8e6) |
| flexbox-safe-overflow-position-002.html | bug (open) | overflow alignment: `safe` / `unsafe` and the fallback of space-* values with negative free space differ from the reference in some positions. Not analysed one by one (2000 pixels differ (largest difference 255); first at (8,54): #008000, the reference has #ffffff) |
| flexbox-safe-overflow-position-003.html | bug (open) | overflow alignment: `safe` / `unsafe` and the fallback of space-* values with negative free space differ from the reference in some positions. Not analysed one by one (1800 pixels differ (largest difference 255); first at (0,64): #008000, the reference has #ffffff) |
| flexbox-safe-overflow-position-004.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (2000 pixels differ (largest difference 255); first at (8,54): #008000, the reference has #ffffff) |
| flexbox-safe-overflow-position-005.html | step 2d | uses text features (2000 pixels differ (largest difference 255); first at (8,54): #008000, the reference has #ffffff) |
| flexbox-table-fixup-001.xhtml | bug (open) | a table as a flex item: its min-content / stretched cross size follows the plain table algorithm with the forced size; Edge sizes the table wrapper box (css-tables-3) differently when it is a flex item. Not analysed one by one (3808 pixels differ (largest difference 255); first at (29,9): #000000, the reference has #ffffff) |
| flexbox-whitespace-handling-001a.xhtml | bug (open) | not analysed one by one (a css-flexbox-1 test that still differs from the reference); the families above and in docs/decisions.md 2b-1 list what is known to be missing (4800 pixels differ (largest difference 255); first at (39,253): #00006c, the reference has #ffffff) |
| flexbox-whitespace-handling-001b.xhtml | bug (open) | not analysed one by one (a css-flexbox-1 test that still differs from the reference); the families above and in docs/decisions.md 2b-1 list what is known to be missing (4800 pixels differ (largest difference 255); first at (39,253): #00006c, the reference has #ffffff) |
| flexbox-writing-mode-002.html | step 2d | uses text features (9600 pixels differ (largest difference 255); first at (9,9): #00ffff, the reference has #ffff00) |
| flexbox-writing-mode-003.html | step 2d | uses text features (4800 pixels differ (largest difference 255); first at (29,9): #ff00ff, the reference has #ffff00) |
| flexbox-writing-mode-005.html | step 2d | uses text features (9600 pixels differ (largest difference 255); first at (9,9): #ff00ff, the reference has #000000) |
| flexbox-writing-mode-006.html | step 2d | uses text features (4800 pixels differ (largest difference 255); first at (29,9): #00ffff, the reference has #000000) |
| flexbox-writing-mode-008.html | step 2d | uses text features (2400 pixels differ (largest difference 255); first at (29,9): #ff00ff, the reference has #ffff00) |
| flexbox-writing-mode-009.html | step 2d | uses text features (4800 pixels differ (largest difference 255); first at (9,9): #00ffff, the reference has #ffff00) |
| flexbox-writing-mode-010.html | step 2d | uses text features (8120 pixels differ (largest difference 255); first at (141,61): #d3d3d3, the reference has #000000) |
| flexbox-writing-mode-011.html | step 2d | uses text features (126032 pixels differ (largest difference 255); first at (139,23): #000000, the reference has #ffffff) |
| flexbox-writing-mode-012.html | step 2d | uses text features (122232 pixels differ (largest difference 255); first at (19,23): #000000, the reference has #ffffff) |
| flexbox-writing-mode-013.html | step 2d | uses text features (55232 pixels differ (largest difference 255); first at (187,23): #ffffff, the reference has #000000) |
| flexbox-writing-mode-014.html | step 2d | uses text features (3360 pixels differ (largest difference 255); first at (21,61): #000000, the reference has #d3d3d3) |
| flexbox-writing-mode-015.html | step 2d | uses text features (17396 pixels differ (largest difference 255); first at (19,23): #ffffff, the reference has #000000) |
| flexbox-writing-mode-slr-row-mix.html | step 2d | uses text features (31456 pixels differ (largest difference 255); first at (19,23): #000000, the reference has #ffffff) |
| flexbox-writing-mode-slr-rtl.html | step 2d | uses text features (9600 pixels differ (largest difference 255); first at (9,9): #ff00ff, the reference has #00ffff) |
| flexbox-writing-mode-slr.html | step 2d | uses text features (9600 pixels differ (largest difference 255); first at (9,9): #00ffff, the reference has #ff00ff) |
| flexbox-writing-mode-srl-row-mix.html | step 2d | uses text features (31456 pixels differ (largest difference 255); first at (19,23): #000000, the reference has #ffffff) |
| flexbox-writing-mode-srl-rtl.html | step 2d | uses text features (9600 pixels differ (largest difference 255); first at (9,9): #ff00ff, the reference has #000000) |
| flexbox-writing-mode-srl.html | step 2d | uses text features (9600 pixels differ (largest difference 255); first at (9,9): #00ffff, the reference has #ffff00) |
| flexbox_align-items-stretch-3.html | step 2d | uses text features (7000 pixels differ (largest difference 255); first at (38,64): #ffffff, the reference has #008000) |
| flexbox_fbfc.html | bug (open) | not analysed one by one (a css-flexbox-1 test that still differs from the reference); the families above and in docs/decisions.md 2b-1 list what is known to be missing (47392 pixels differ (largest difference 255); first at (150,0): #ffcc00, the reference has #ffffff) |
| flexbox_flex-0-0-N-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (2816 pixels differ (largest difference 255); first at (265,33): #000000, the reference has #ffffff) |
| flexbox_flex-0-0-N.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (2816 pixels differ (largest difference 255); first at (265,33): #000000, the reference has #0000ff) |
| flexbox_flex-0-1-N-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (6912 pixels differ (largest difference 255); first at (217,33): #000000, the reference has #ffffff) |
| flexbox_flex-0-1-N.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (2816 pixels differ (largest difference 255); first at (265,33): #000000, the reference has #0000ff) |
| flexbox_flex-0-1-Npercent-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (6912 pixels differ (largest difference 255); first at (217,33): #000000, the reference has #ffffff) |
| flexbox_flex-0-1-auto-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (2432 pixels differ (largest difference 255); first at (65,33): #000000, the reference has #ffff00) |
| flexbox_flex-0-N-N-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (6912 pixels differ (largest difference 255); first at (217,33): #000000, the reference has #ffffff) |
| flexbox_flex-0-N-N.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (2816 pixels differ (largest difference 255); first at (265,33): #000000, the reference has #0000ff) |
| flexbox_flex-0-N-Npercent-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (6912 pixels differ (largest difference 255); first at (217,33): #000000, the reference has #ffffff) |
| flexbox_flex-0-N-auto-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (2432 pixels differ (largest difference 255); first at (65,33): #000000, the reference has #ffff00) |
| flexbox_flex-0-auto.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (15360 pixels differ (largest difference 255); first at (73,33): #0000ff, the reference has #ffffff) |
| flexbox_flex-1-0-N-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (2816 pixels differ (largest difference 255); first at (265,33): #000000, the reference has #ffffff) |
| flexbox_flex-1-1-N-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (6912 pixels differ (largest difference 255); first at (217,33): #000000, the reference has #ffffff) |
| flexbox_flex-1-1-Npercent-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (6912 pixels differ (largest difference 255); first at (217,33): #000000, the reference has #ffffff) |
| flexbox_flex-1-1-auto-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (2432 pixels differ (largest difference 255); first at (65,33): #000000, the reference has #ffff00) |
| flexbox_flex-1-N-N-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (6912 pixels differ (largest difference 255); first at (217,33): #000000, the reference has #ffffff) |
| flexbox_flex-1-N-Npercent-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (6912 pixels differ (largest difference 255); first at (217,33): #000000, the reference has #ffffff) |
| flexbox_flex-1-N-auto-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (2432 pixels differ (largest difference 255); first at (65,33): #000000, the reference has #ffff00) |
| flexbox_flex-N-0-N-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (2816 pixels differ (largest difference 255); first at (265,33): #000000, the reference has #ffffff) |
| flexbox_flex-N-1-N-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (6912 pixels differ (largest difference 255); first at (217,33): #000000, the reference has #ffffff) |
| flexbox_flex-N-1-Npercent-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (6912 pixels differ (largest difference 255); first at (217,33): #000000, the reference has #ffffff) |
| flexbox_flex-N-1-auto-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (2432 pixels differ (largest difference 255); first at (65,33): #000000, the reference has #ffff00) |
| flexbox_flex-N-N-N-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (6912 pixels differ (largest difference 255); first at (217,33): #000000, the reference has #ffffff) |
| flexbox_flex-N-N-Npercent-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (6912 pixels differ (largest difference 255); first at (217,33): #000000, the reference has #ffffff) |
| flexbox_flex-N-N-auto-shrink.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (2432 pixels differ (largest difference 255); first at (65,33): #000000, the reference has #ffff00) |
| flexbox_flex-initial.html | limit (test platform) | the test depends on the width of real text in the default font: with the test platform's 1em glyphs a word ("three", "four") is wider than the box the item shrinks to, so the automatic minimum size (css-flexbox-1 4.5) keeps it wider than the reference shows (15360 pixels differ (largest difference 255); first at (73,33): #0000ff, the reference has #ffffff) |
| flexbox_object.html | limit (placeholder boxes) | canvas, video, iframe and object are grey placeholder boxes in layer 2 (no content, no intrinsic size), and fieldset has no special layout; the reference needs the real thing (104992 pixels differ (largest difference 204); first at (8,8): #767676, the reference has #000000) |
| flexbox_rtl-direction.html | step 2d | uses text features (4096 pixels differ (largest difference 255); first at (647,33): #000000, the reference has #ffffff) |
| flexbox_rtl-flow-reverse.html | step 2d | uses text features (6656 pixels differ (largest difference 255); first at (25,33): #000000, the reference has #ffffff) |
| flexbox_rtl-flow.html | step 2d | uses text features (6656 pixels differ (largest difference 255); first at (25,33): #000000, the reference has #ffffff) |
| flexbox_rtl-order.html | step 2d | uses text features (6656 pixels differ (largest difference 255); first at (25,49): #ffffff, the reference has #000000) |
| flexbox_writing_mode_vertical_lays_out_contents_from_top_to_bottom.html | step 2d | uses text features (40000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| flexitem-stretch-range.html | step 2e | uses svg, scrolling, sticky or form controls (78778 pixels differ (largest difference 137); first at (13,162): #e0e0e0, the reference has #767676) |
| gap-001-rtl.html | step 2d | uses text features (8000 pixels differ (largest difference 128); first at (8,64): #808080, the reference has #008000) |
| gap-003-rtl.html | step 2d | uses text features (3200 pixels differ (largest difference 128); first at (390,64): #008000, the reference has #808080) |
| gap-004-rtl.html | step 2d | uses text features (13024 pixels differ (largest difference 255); first at (88,64): #008000, the reference has #ffffff) |
| gap-006-rtl.html | step 2d | uses text features (2992 pixels differ (largest difference 255); first at (68,64): #008000, the reference has #ffffff) |
| gap-007-lr.html | step 2d | uses text features (10104 pixels differ (largest difference 255); first at (188,64): #808080, the reference has #008000) |
| gap-007-ltr.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10104 pixels differ (largest difference 255); first at (188,64): #808080, the reference has #008000) |
| gap-007-rl.html | step 2d | uses text features (10104 pixels differ (largest difference 255); first at (188,64): #808080, the reference has #008000) |
| gap-007-rtl.html | step 2d | uses text features (4320 pixels differ (largest difference 255); first at (188,64): #808080, the reference has #008000) |
| gap-collapse.html | step 2c | uses painting effects (44200 pixels differ (largest difference 255); first at (108,8): #ffffff, the reference has #0000ff) |
| gap-percentage-wrap.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| grid-flex-item-001.html | step 2b | uses grid (step 2b-2) (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| grid-flex-item-002.html | step 2b | uses grid (step 2b-2) (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| image-nested-within-definite-column-flexbox.html | step 2e | uses svg, scrolling, sticky or form controls (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| inline-flexbox-vertical-rl-image-flexitem-crash-print.html | step 2d | uses text features (/common/blank.html answered HTTP 404) |
| intrinsic-size/auto-min-size-001.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (10000 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| intrinsic-size/col-wrap-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (5000 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| intrinsic-size/col-wrap-002.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (5000 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| intrinsic-size/col-wrap-003.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (4000 pixels differ (largest difference 255); first at (68,64): #ff0000, the reference has #008000) |
| intrinsic-size/col-wrap-006.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (5000 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| intrinsic-size/col-wrap-007.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (3000 pixels differ (largest difference 255); first at (78,64): #ff0000, the reference has #008000) |
| intrinsic-size/col-wrap-008.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (3000 pixels differ (largest difference 255); first at (78,64): #ff0000, the reference has #008000) |
| intrinsic-size/col-wrap-013.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (2500 pixels differ (largest difference 255); first at (83,64): #ffffff, the reference has #008000) |
| intrinsic-size/col-wrap-014.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (2500 pixels differ (largest difference 255); first at (83,64): #ffffff, the reference has #008000) |
| intrinsic-size/col-wrap-020.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (9000 pixels differ (largest difference 255); first at (18,64): #ff0000, the reference has #008000) |
| intrinsic-size/row-002.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| intrinsic-size/row-003.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| intrinsic-size/row-wrap-002.tentative.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (1000 pixels differ (largest difference 255); first at (98,64): #ff0000, the reference has #008000) |
| justify-content-001.htm | step 2c | uses painting effects (14800 pixels differ (largest difference 255); first at (8,96): #ffffff, the reference has #0000ff) |
| justify-content-002.htm | step 2c | uses painting effects (15000 pixels differ (largest difference 255); first at (158,96): #ffffff, the reference has #ffa500) |
| justify-content-003.htm | step 2c | uses painting effects (15000 pixels differ (largest difference 255); first at (8,96): #ffffff, the reference has #0000ff) |
| justify-content-004.htm | step 2c | uses painting effects (14800 pixels differ (largest difference 255); first at (84,96): #ffffff, the reference has #0000ff) |
| justify-content-005.htm | step 2c | uses painting effects (14800 pixels differ (largest difference 255); first at (8,96): #ffffff, the reference has #0000ff) |
| justify-content-sideways-001.html | step 2d | uses text features (10800 pixels differ (largest difference 255); first at (0,64): #008000, the reference has #ffffff) |
| multiline-reverse-wrap-baseline.html | bug (open) | baseline alignment (css-flexbox-1 8.5, 9.4): items with different font sizes or in vertical lines; the references use offsets worked out with real font metrics, the first baseline of a container is the first item's. Not analysed one by one (22400 pixels differ (largest difference 255); first at (108,8): #000000, the reference has #aaaaaa) |
| multiline-shrink-to-fit.html | bug (open) | intrinsic sizes of flex containers (9.9): multi-line and column containers use the simple formula of the web-compatible algorithm, items with flex-basis/min-content interplay are approximated. Not analysed one by one (250 pixels differ (largest difference 85); first at (108,292): #ffffff, the reference has #aaaaaa) |
| nested-flex-image-loading-invalidates-intrinsic-sizes.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (8400 pixels differ (largest difference 255); first at (48,48): #ffffff, the reference has #008000) |
| order/order-abs-children-painting-order.html | bug (open) | out-of-flow descendants of a nested flex container: their containing block / static position when an inset or `order` is involved is not matched. Not analysed one by one (10000 pixels differ (largest difference 255); first at (8,48): #ff0000, the reference has #008000) |
| overflow-area-001.html | step 2e | uses svg, scrolling, sticky or form controls (20400 pixels differ (largest difference 255); first at (217,91): #0000ff, the reference has #00ffff) |
| overflow-area-002.html | step 2d | uses text features (20400 pixels differ (largest difference 255); first at (127,91): #ffffff, the reference has #00ffff) |
| overflow-top-left.html | step 2d | uses text features (31760 pixels differ (largest difference 255); first at (310,48): #000000, the reference has #ffffff) |
| percentage-descendant-of-anonymous-flex-item.html | step 2e | uses svg, scrolling, sticky or form controls (20728 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #767676) |
| percentage-heights-015.html | step 2e | uses svg, scrolling, sticky or form controls (55000 pixels differ (largest difference 255); first at (8,164): #ff0000, the reference has #008000) |
| percentage-heights-020.html | limit (placeholder boxes) | canvas, video, iframe and object are grey placeholder boxes in layer 2 (no content, no intrinsic size), and fieldset has no special layout; the reference needs the real thing (31916 pixels differ (largest difference 255); first at (8,64): #767676, the reference has #008000) |
| percentage-heights-022.html | limit (placeholder boxes) | canvas, video, iframe and object are grey placeholder boxes in layer 2 (no content, no intrinsic size), and fieldset has no special layout; the reference needs the real thing (121935 pixels differ (largest difference 255); first at (8,64): #767676, the reference has #008000) |
| percentage-heights-023.html | limit (placeholder boxes) | canvas, video, iframe and object are grey placeholder boxes in layer 2 (no content, no intrinsic size), and fieldset has no special layout; the reference needs the real thing (42332 pixels differ (largest difference 255); first at (8,64): #767676, the reference has #008000) |
| percentage-heights-024.html | limit (placeholder boxes) | canvas, video, iframe and object are grey placeholder boxes in layer 2 (no content, no intrinsic size), and fieldset has no special layout; the reference needs the real thing (1916 pixels differ (largest difference 118); first at (8,64): #767676, the reference has #008000) |
| percentage-max-height-002.html | step 2e | uses svg, scrolling, sticky or form controls (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| percentage-max-height-003.html | step 2e | uses svg, scrolling, sticky or form controls (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| percentage-widths-001.html | bug (open) | definite sizes (css-flexbox-1 9.8) for percentages inside items: a stretched item or an item of an auto-height container is not always definite for its children; and percentage widths wait for the shrunk size. Not analysed one by one (15552 pixels differ (largest difference 255); first at (108,8): #008000, the reference has #000000) |
| position-relative-percentage-top-002.html | bug (open) | definite sizes (css-flexbox-1 9.8) for percentages inside items: a stretched item or an item of an auto-height container is not always definite for its children; and percentage widths wait for the shrunk size. Not analysed one by one (20000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| relayout-intrinsic-block-size.html | bug (open) | flex items that are replaced elements with an aspect ratio: the flex-specific rules (transferred size suggestion 4.5, flex base size from the ratio 9.2 B, stretching an image keeping its ratio) are not implemented; only the ratio of the replaced box itself is used (11834 pixels differ (largest difference 255); first at (8,48): #767676, the reference has #008000) |
| select-element-multiple.html | step 2e | uses svg, scrolling, sticky or form controls (448 pixels differ (largest difference 255); first at (8,8): #767676, the reference has #ffffff) |
| select-element-zero-height-001.html | step 2e | uses svg, scrolling, sticky or form controls (3150 pixels differ (largest difference 255); first at (9,9): #767676, the reference has #000000) |
| select-element-zero-height-002.html | step 2e | uses svg, scrolling, sticky or form controls (3150 pixels differ (largest difference 255); first at (9,9): #767676, the reference has #000000) |
| stretch-requires-computed-auto-size.html | bug (open) | definite sizes (css-flexbox-1 9.8) for percentages inside items: a stretched item or an item of an auto-height container is not always definite for its children; and percentage widths wait for the shrunk size. Not analysed one by one (10000 pixels differ (largest difference 255); first at (108,64): #ff0000, the reference has #ffffff) |
| svg-no-natural-size-grandchild.html | step 2e | uses svg, scrolling, sticky or form controls (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| svg-root-as-flex-item-001.html | step 2e | uses svg, scrolling, sticky or form controls (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| svg-root-as-flex-item-002.html | step 2e | uses svg, scrolling, sticky or form controls (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| svg-root-as-flex-item-003.html | step 2e | uses svg, scrolling, sticky or form controls (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| svg-root-as-flex-item-004.html | step 2e | uses svg, scrolling, sticky or form controls (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| svg-root-as-flex-item-005.html | step 2e | uses svg, scrolling, sticky or form controls (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| table-as-item-fixed-min-width-3.html | bug (open) | a table as a flex item: its min-content / stretched cross size follows the plain table algorithm with the forced size; Edge sizes the table wrapper box (css-tables-3) differently when it is a flex item. Not analysed one by one (5000 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| table-as-item-flex-cross-size.html | bug (open) | a table as a flex item: its min-content / stretched cross size follows the plain table algorithm with the forced size; Edge sizes the table wrapper box (css-tables-3) differently when it is a flex item. Not analysed one by one (3000 pixels differ (largest difference 255); first at (8,164): #008000, the reference has #ffffff) |
| table-as-item-min-content-height-1.tentative.html | bug (open) | a table as a flex item: its min-content / stretched cross size follows the plain table algorithm with the forced size; Edge sizes the table wrapper box (css-tables-3) differently when it is a flex item. Not analysed one by one (5000 pixels differ (largest difference 255); first at (8,114): #ff0000, the reference has #008000) |
| table-as-item-min-content-height-2.tentative.html | bug (open) | a table as a flex item: its min-content / stretched cross size follows the plain table algorithm with the forced size; Edge sizes the table wrapper box (css-tables-3) differently when it is a flex item. Not analysed one by one (5000 pixels differ (largest difference 255); first at (8,114): #ff0000, the reference has #008000) |
| table-as-item-stretch-cross-size-2.html | bug (open) | a table as a flex item: its min-content / stretched cross size follows the plain table algorithm with the forced size; Edge sizes the table wrapper box (css-tables-3) differently when it is a flex item. Not analysed one by one (3000 pixels differ (largest difference 255); first at (8,164): #008000, the reference has #ffffff) |
| table-as-item-stretch-cross-size-5.html | bug (open) | a table as a flex item: its min-content / stretched cross size follows the plain table algorithm with the forced size; Edge sizes the table wrapper box (css-tables-3) differently when it is a flex item. Not analysed one by one (2000 pixels differ (largest difference 255); first at (8,144): #ff0000, the reference has #008000) |
| table-as-item-stretch-cross-size.html | bug (open) | a table as a flex item: its min-content / stretched cross size follows the plain table algorithm with the forced size; Edge sizes the table wrapper box (css-tables-3) differently when it is a flex item. Not analysed one by one (3000 pixels differ (largest difference 255); first at (8,164): #008000, the reference has #ffffff) |

## 通过

- abspos/abspos-autopos-htb-ltr.html
- abspos/abspos-autopos-htb-rtl.html
- abspos/abspos-autopos-vlr-ltr.html
- abspos/abspos-autopos-vlr-rtl.html
- abspos/abspos-autopos-vrl-ltr.html
- abspos/abspos-autopos-vrl-rtl.html
- abspos/flex-abspos-align-self-safe-outer-cb-003.tentative.html
- abspos/flex-abspos-staticpos-align-self-safe-003.html
- abspos/flex-abspos-staticpos-justify-self-001.html
- abspos/flex-abspos-staticpos-margin-001.html
- abspos/flex-abspos-staticpos-margin-002.html
- abspos/flex-abspos-staticpos-margin-003.html
- abspos/flexbox-abspos-child-001a.html
- abspos/flexbox-abspos-child-001b.html
- abspos/flexbox-abspos-child-002.html
- abspos/flexbox_absolute-atomic.html
- abspos/flexbox_inline-abspos.html
- abspos/position-absolute-006.html
- abspos/position-absolute-007.html
- abspos/position-absolute-008.html
- abspos/position-absolute-009.html
- abspos/position-absolute-010.html
- abspos/position-absolute-011.html
- abspos/position-absolute-containing-block-001.html
- abspos/position-absolute-containing-block-002.html
- align-content-006.htm
- align-content-wrap-004.html
- align-content_center.html
- align-content_flex-end.html
- align-content_flex-start.html
- align-content_space-around.html
- align-content_space-between.html
- align-content_stretch.html
- align-items-005.htm
- align-items-006.html
- align-items-007.html
- align-items-009.html
- align-items-baseline-overflow-non-visible.html
- align-self-001.html
- align-self-002.html
- align-self-003.html
- align-self-004.html
- align-self-005.html
- align-self-007.html
- align-self-008.html
- align-self-009.html
- align-self-011.html
- align-self-012.html
- align-self-013.html
- align-self-015.html
- alignment/flex-align-baseline-column-rtl-direction.html
- anonymous-block.html
- anonymous-flex-item-002.html
- anonymous-flex-item-004.html
- aspect-ratio-intrinsic-size-007.html
- auto-height-column-with-border-and-padding.html
- auto-height-with-flex.html
- balance/balance-004.html
- balance/balance-min-line-count-007.html
- balance/balance-min-line-count-008.html
- balance/balance-percentage-size-002.html
- balance/webkit-box-no-balance.html
- break-nested-float-in-flex-item-001-print.html
- break-nested-float-in-flex-item-002-print.html
- column-flex-child-center-with-indefinite-height.html
- column-flex-child-with-max-width.html
- css-flexbox-height-animation-stretch.html
- css-flexbox-img-expand-evenly.html
- display-flex-001.htm
- empty-flex-box-and-margin-collapsing.html
- fit-content-item-001.html
- fit-content-item-002.html
- fit-content-item-003.html
- fit-content-item-004.html
- flex-001.htm
- flex-002.htm
- flex-003.htm
- flex-004.htm
- flex-abspos-inset-cross-size-001.html
- flex-align-content-center.html
- flex-align-content-end.html
- flex-align-content-space-around.html
- flex-align-content-space-between.html
- flex-align-content-start.html
- flex-aspect-ratio-img-column-001.html
- flex-aspect-ratio-img-column-002.html
- flex-aspect-ratio-img-column-003.html
- flex-aspect-ratio-img-column-005.html
- flex-aspect-ratio-img-column-006.html
- flex-aspect-ratio-img-column-007.html
- flex-aspect-ratio-img-column-009.html
- flex-aspect-ratio-img-column-016.html
- flex-aspect-ratio-img-row-001.html
- flex-aspect-ratio-img-row-002.html
- flex-aspect-ratio-img-row-003.html
- flex-aspect-ratio-img-row-012.html
- flex-aspect-ratio-img-row-014.html
- flex-aspect-ratio-img-vert-lr.html
- flex-base-size-ignores-max-width.html
- flex-basis-001.html
- flex-basis-002.html
- flex-basis-003.html
- flex-basis-004.html
- flex-basis-005.html
- flex-basis-006.html
- flex-basis-007.html
- flex-basis-008.html
- flex-basis-010.html
- flex-basis-011.html
- flex-basis-012.html
- flex-basis-item-margins-001.html
- flex-box-wrap.html
- flex-column-reverse-multiline-item-position.html
- flex-container-margin.html
- flex-cross-size-border-box-001.html
- flex-direction-column-reverse.html
- flex-direction-column.html
- flex-direction-row-reverse.html
- flex-direction-row-vertical.html
- flex-direction.html
- flex-flow-001.html
- flex-flow-002.html
- flex-flow-003.html
- flex-flow-004.html
- flex-flow-005.html
- flex-flow-006.html
- flex-flow-007.html
- flex-flow-008.html
- flex-flow-009.html
- flex-flow-010.html
- flex-flow-011.html
- flex-flow-012.html
- flex-grow-002.html
- flex-grow-003.html
- flex-grow-004.html
- flex-grow-005.html
- flex-grow-006.html
- flex-grow-007.html
- flex-grow-008.html
- flex-height-min-content.html
- flex-inline.html
- flex-item-and-percentage-abspos.html
- flex-item-content-is-min-width-max-content.html
- flex-item-max-width-min-content-002.html
- flex-item-max-width-min-content.html
- flex-item-min-height-min-content-overflow.html
- flex-item-min-height-min-content.html
- flex-item-min-width-min-content-max-width.html
- flex-item-min-width-min-content.html
- flex-item-percentage-in-nested-flex-bsize-001.html
- flex-item-percentage-in-nested-flex-bsize-002.html
- flex-item-position-relative-001.html
- flex-item-vertical-align.html
- flex-item-z-ordering-001.html
- flex-item-z-ordering-002.html
- flex-lines/multi-line-wrap-reverse-row-reverse.html
- flex-lines/multi-line-wrap-with-row-reverse.html
- flex-margin-no-collapse.html
- flex-minimum-height-flex-items-001.xht
- flex-minimum-height-flex-items-002.xht
- flex-minimum-height-flex-items-003.xht
- flex-minimum-height-flex-items-004.xht
- flex-minimum-height-flex-items-005.xht
- flex-minimum-height-flex-items-006.xht
- flex-minimum-height-flex-items-007.xht
- flex-minimum-height-flex-items-008.xht
- flex-minimum-height-flex-items-011.xht
- flex-minimum-height-flex-items-013.html
- flex-minimum-height-flex-items-014.html
- flex-minimum-height-flex-items-015.html
- flex-minimum-height-flex-items-016.html
- flex-minimum-height-flex-items-017.html
- flex-minimum-height-flex-items-018.html
- flex-minimum-height-flex-items-020.html
- flex-minimum-height-flex-items-021.html
- flex-minimum-height-flex-items-022.html
- flex-minimum-height-flex-items-023.html
- flex-minimum-height-flex-items-024.html
- flex-minimum-height-flex-items-026.html
- flex-minimum-height-flex-items-027.html
- flex-minimum-height-flex-items-028.html
- flex-minimum-height-flex-items-029.html
- flex-minimum-height-flex-items-030.html
- flex-minimum-width-flex-items-001.xht
- flex-minimum-width-flex-items-002.xht
- flex-minimum-width-flex-items-003.xht
- flex-minimum-width-flex-items-004.xht
- flex-minimum-width-flex-items-005.xht
- flex-minimum-width-flex-items-006.xht
- flex-minimum-width-flex-items-007.xht
- flex-minimum-width-flex-items-008.xht
- flex-minimum-width-flex-items-009.html
- flex-minimum-width-flex-items-010.html
- flex-minimum-width-flex-items-011.html
- flex-minimum-width-flex-items-012.html
- flex-order-last-baseline-multiple.html
- flex-order-last-baseline.html
- flex-order-wrap-reverse-baseline.html
- flex-order.html
- flex-shrink-001.html
- flex-shrink-002.html
- flex-shrink-003.html
- flex-shrink-004.html
- flex-shrink-005.html
- flex-shrink-006.html
- flex-shrink-007.html
- flex-shrink-008.html
- flex-vertical-align-effect.html
- flex-wrap-002.html
- flex-wrap-003.html
- flex-wrap-004.html
- flex-wrap-005.html
- flexbox-align-items-center-nested-001.html
- flexbox-align-self-baseline-compatability.html
- flexbox-align-self-baseline-horiz-002.xhtml
- flexbox-align-self-baseline-horiz-003.xhtml
- flexbox-align-self-baseline-horiz-004.xhtml
- flexbox-align-self-baseline-horiz-007.xhtml
- flexbox-align-self-horiz-005.xhtml
- flexbox-align-self-stretch-vert-001.html
- flexbox-align-self-stretch-vert-002.html
- flexbox-align-self-vert-001.xhtml
- flexbox-align-self-vert-003.xhtml
- flexbox-align-self-vert-004.xhtml
- flexbox-anonymous-items-001.html
- flexbox-baseline-align-self-baseline-horiz-001.html
- flexbox-baseline-align-self-baseline-vert-001.html
- flexbox-baseline-empty-001a.html
- flexbox-baseline-empty-001b.html
- flexbox-baseline-multi-item-vert-001a.html
- flexbox-baseline-multi-item-vert-001b.html
- flexbox-baseline-multi-line-horiz-003.html
- flexbox-baseline-nested-001.html
- flexbox-baseline-single-item-001a.html
- flexbox-baseline-single-item-001b.html
- flexbox-basic-block-horiz-001.xhtml
- flexbox-basic-block-vert-001.xhtml
- flexbox-basic-canvas-horiz-001.xhtml
- flexbox-basic-canvas-horiz-001v.xhtml
- flexbox-basic-fieldset-horiz-001.xhtml
- flexbox-basic-iframe-horiz-001.xhtml
- flexbox-basic-img-horiz-001.xhtml
- flexbox-basic-video-horiz-001.xhtml
- flexbox-break-request-horiz-001a.html
- flexbox-break-request-horiz-001b.html
- flexbox-break-request-horiz-002a.html
- flexbox-break-request-horiz-002b.html
- flexbox-break-request-vert-001a.html
- flexbox-break-request-vert-001b.html
- flexbox-break-request-vert-002a.html
- flexbox-break-request-vert-002b.html
- flexbox-collapsed-item-horiz-001.html
- flexbox-column-row-gap-001.html
- flexbox-column-row-gap-003.html
- flexbox-definite-sizes-001.html
- flexbox-definite-sizes-006.html
- flexbox-flex-basis-content-002a.html
- flexbox-flex-basis-content-002b.html
- flexbox-flex-basis-content-003a.html
- flexbox-flex-basis-content-003b.html
- flexbox-flex-basis-content-004a.html
- flexbox-flex-basis-content-004b.html
- flexbox-flex-direction-column-percentage-ignored.html
- flexbox-flex-direction-column-reverse.htm
- flexbox-flex-direction-column.htm
- flexbox-flex-direction-default.htm
- flexbox-flex-direction-row-reverse.htm
- flexbox-flex-direction-row.htm
- flexbox-flex-flow-001.html
- flexbox-flex-flow-002.html
- flexbox-flex-wrap-default.htm
- flexbox-flex-wrap-flexing-002.html
- flexbox-flex-wrap-flexing.html
- flexbox-flex-wrap-horiz-001.html
- flexbox-flex-wrap-horiz-002.html
- flexbox-flex-wrap-nowrap.htm
- flexbox-flex-wrap-vert-001.html
- flexbox-flex-wrap-vert-002.html
- flexbox-flex-wrap-wrap-reverse.htm
- flexbox-flex-wrap-wrap.htm
- flexbox-gap-position-absolute.html
- flexbox-items-as-stacking-contexts-002.html
- flexbox-items-as-stacking-contexts-003.html
- flexbox-justify-content-horiz-001a.xhtml
- flexbox-justify-content-horiz-001b.xhtml
- flexbox-justify-content-horiz-002.xhtml
- flexbox-justify-content-horiz-003.xhtml
- flexbox-justify-content-horiz-004.xhtml
- flexbox-justify-content-horiz-005.xhtml
- flexbox-justify-content-vert-001a.xhtml
- flexbox-justify-content-vert-001b.xhtml
- flexbox-justify-content-vert-002.xhtml
- flexbox-justify-content-vert-003.xhtml
- flexbox-justify-content-vert-004.xhtml
- flexbox-justify-content-vert-005.xhtml
- flexbox-justify-content-vert-006.xhtml
- flexbox-margin-auto-horiz-001.xhtml
- flexbox-margin-auto-horiz-002.xhtml
- flexbox-mbp-horiz-001-reverse.xhtml
- flexbox-mbp-horiz-001-rtl-reverse.xhtml
- flexbox-mbp-horiz-001-rtl.xhtml
- flexbox-mbp-horiz-001.xhtml
- flexbox-mbp-horiz-002a.xhtml
- flexbox-mbp-horiz-002b.xhtml
- flexbox-mbp-horiz-003-reverse.xhtml
- flexbox-mbp-horiz-003.xhtml
- flexbox-mbp-horiz-003v.xhtml
- flexbox-mbp-horiz-004.xhtml
- flexbox-min-height-auto-001.html
- flexbox-min-height-auto-002b.html
- flexbox-min-width-auto-001.html
- flexbox-min-width-auto-003.html
- flexbox-min-width-auto-004.html
- flexbox-order-only-flexitems.html
- flexbox-overflow-horiz-002.html
- flexbox-overflow-horiz-003.html
- flexbox-overflow-horiz-004.html
- flexbox-overflow-horiz-005.html
- flexbox-overflow-padding-001.html
- flexbox-overflow-padding-002.html
- flexbox-overflow-vert-001.html
- flexbox-overflow-vert-002.html
- flexbox-overflow-vert-003.html
- flexbox-overflow-vert-004.html
- flexbox-overflow-vert-005.html
- flexbox-paint-ordering-001.xhtml
- flexbox-paint-ordering-003.html
- flexbox-root-node-001a.html
- flexbox-root-node-001b.html
- flexbox-safe-overflow-position-001.html
- flexbox-safe-overflow-position-006.html
- flexbox-single-line-clamp-1.html
- flexbox-single-line-clamp-2.html
- flexbox-single-line-clamp-3.html
- flexbox-sizing-horiz-001.xhtml
- flexbox-sizing-horiz-002.xhtml
- flexbox-sizing-vert-001.xhtml
- flexbox-sizing-vert-002.xhtml
- flexbox-vert-lr-with-img.html
- flexbox-whitespace-handling-002.xhtml
- flexbox-with-multi-column-property.html
- flexbox-with-pseudo-elements-001.html
- flexbox-with-pseudo-elements-002.html
- flexbox-with-pseudo-elements-003.html
- flexbox-writing-mode-001.html
- flexbox-writing-mode-004.html
- flexbox-writing-mode-007.html
- flexbox-writing-mode-016.html
- flexbox_align-content-center.html
- flexbox_align-content-flexend.html
- flexbox_align-content-flexstart.html
- flexbox_align-content-spacearound.html
- flexbox_align-content-spacebetween.html
- flexbox_align-content-stretch-2.html
- flexbox_align-content-stretch.html
- flexbox_align-items-baseline.html
- flexbox_align-items-center-2.html
- flexbox_align-items-center-3.html
- flexbox_align-items-center.html
- flexbox_align-items-flexend-2.html
- flexbox_align-items-flexend.html
- flexbox_align-items-flexstart-2.html
- flexbox_align-items-flexstart.html
- flexbox_align-items-stretch-2.html
- flexbox_align-items-stretch-4.html
- flexbox_align-items-stretch.html
- flexbox_align-self-auto.html
- flexbox_align-self-baseline.html
- flexbox_align-self-center.html
- flexbox_align-self-flexend.html
- flexbox_align-self-flexstart.html
- flexbox_align-self-stretch.html
- flexbox_block.html
- flexbox_box-clear.html
- flexbox_columns-flexitems-2.html
- flexbox_columns-flexitems.html
- flexbox_direction-column-reverse.html
- flexbox_direction-column.html
- flexbox_direction-row-reverse.html
- flexbox_display.html
- flexbox_fbfc2.html
- flexbox_first-line.html
- flexbox_flex-0-0-0-unitless.html
- flexbox_flex-0-0-0.html
- flexbox_flex-0-0-1-unitless-basis.html
- flexbox_flex-0-0-N-unitless-basis.html
- flexbox_flex-0-0-Npercent-shrink.html
- flexbox_flex-0-0-Npercent.html
- flexbox_flex-0-0-auto-shrink.html
- flexbox_flex-0-0-auto.html
- flexbox_flex-0-0.html
- flexbox_flex-0-1-0-unitless.html
- flexbox_flex-0-1-0.html
- flexbox_flex-0-1-1-unitless-basis.html
- flexbox_flex-0-1-N-unitless-basis.html
- flexbox_flex-0-1-Npercent.html
- flexbox_flex-0-1-auto.html
- flexbox_flex-0-1.html
- flexbox_flex-0-N-0-unitless.html
- flexbox_flex-0-N-0.html
- flexbox_flex-0-N-Npercent.html
- flexbox_flex-0-N-auto.html
- flexbox_flex-0-N.html
- flexbox_flex-1-0-0-unitless.html
- flexbox_flex-1-0-0.html
- flexbox_flex-1-0-N.html
- flexbox_flex-1-0-Npercent-shrink.html
- flexbox_flex-1-0-Npercent.html
- flexbox_flex-1-0-auto-shrink.html
- flexbox_flex-1-0-auto.html
- flexbox_flex-1-0.html
- flexbox_flex-1-1-0-unitless.html
- flexbox_flex-1-1-0.html
- flexbox_flex-1-1-N.html
- flexbox_flex-1-1-Npercent.html
- flexbox_flex-1-1-auto.html
- flexbox_flex-1-1.html
- flexbox_flex-1-N-0-unitless.html
- flexbox_flex-1-N-0.html
- flexbox_flex-1-N-N.html
- flexbox_flex-1-N-Npercent.html
- flexbox_flex-1-N-auto.html
- flexbox_flex-1-N.html
- flexbox_flex-N-0-0-unitless.html
- flexbox_flex-N-0-0.html
- flexbox_flex-N-0-N.html
- flexbox_flex-N-0-Npercent-shrink.html
- flexbox_flex-N-0-Npercent.html
- flexbox_flex-N-0-auto-shrink.html
- flexbox_flex-N-0-auto.html
- flexbox_flex-N-0.html
- flexbox_flex-N-1-0-unitless.html
- flexbox_flex-N-1-0.html
- flexbox_flex-N-1-N.html
- flexbox_flex-N-1-Npercent.html
- flexbox_flex-N-1-auto.html
- flexbox_flex-N-1.html
- flexbox_flex-N-N-0-unitless.html
- flexbox_flex-N-N-0.html
- flexbox_flex-N-N-N.html
- flexbox_flex-N-N-Npercent.html
- flexbox_flex-N-N-auto.html
- flexbox_flex-N-N.html
- flexbox_flex-auto.html
- flexbox_flex-basis-shrink.html
- flexbox_flex-basis.html
- flexbox_flex-formatting-interop.html
- flexbox_flex-initial-2.html
- flexbox_flex-natural-mixed-basis-auto.html
- flexbox_flex-natural-mixed-basis.html
- flexbox_flex-natural-variable-auto-basis.html
- flexbox_flex-natural-variable-zero-basis.html
- flexbox_flex-natural.html
- flexbox_flex-none-wrappable-content.html
- flexbox_flex-none.html
- flexbox_flow-column-reverse-wrap-reverse.html
- flexbox_flow-column-reverse-wrap.html
- flexbox_flow-column-wrap-reverse.html
- flexbox_flow-column-wrap.html
- flexbox_flow-row-wrap-reverse.html
- flexbox_flow-row-wrap.html
- flexbox_generated-flex.html
- flexbox_generated-nested-flex.html
- flexbox_generated.html
- flexbox_inline-float.html
- flexbox_inline.html
- flexbox_item-bottom-float.html
- flexbox_item-clear.html
- flexbox_item-float.html
- flexbox_item-top-float.html
- flexbox_item-vertical-align.html
- flexbox_justifycontent-center.html
- flexbox_justifycontent-end-rtl.html
- flexbox_justifycontent-end.html
- flexbox_justifycontent-flex-end.html
- flexbox_justifycontent-flex-start.html
- flexbox_justifycontent-left-001.html
- flexbox_justifycontent-left-002.html
- flexbox_justifycontent-right-001.html
- flexbox_justifycontent-right-002.html
- flexbox_justifycontent-spacearound-negative.html
- flexbox_justifycontent-spacearound-only.html
- flexbox_justifycontent-spacearound.html
- flexbox_justifycontent-spacebetween-negative.html
- flexbox_justifycontent-spacebetween-only.html
- flexbox_justifycontent-spacebetween.html
- flexbox_justifycontent-start-rtl.html
- flexbox_justifycontent-start.html
- flexbox_justifycontent-stretch.html
- flexbox_margin-auto-overflow.html
- flexbox_margin-auto.html
- flexbox_margin-left-ex.html
- flexbox_margin.html
- flexbox_nested-flex.html
- flexbox_order-abspos-space-around.html
- flexbox_order-box.html
- flexbox_order-noninteger-invalid.html
- flexbox_order.html
- flexbox_quirks_body.html
- flexbox_rowspan-overflow-automatic.html
- flexbox_rowspan-overflow.html
- flexbox_rowspan.html
- flexbox_stf-abspos.html
- flexbox_stf-fixpos.html
- flexbox_stf-float.html
- flexbox_stf-inline-block.html
- flexbox_stf-table-caption.html
- flexbox_stf-table-cell.html
- flexbox_stf-table-row-group.html
- flexbox_stf-table-row.html
- flexbox_stf-table-singleline-2.html
- flexbox_stf-table-singleline.html
- flexbox_stf-table.html
- flexbox_table-fixed-layout.html
- flexbox_visibility-collapse-line-wrapping.html
- flexbox_visibility-collapse.html
- flexbox_width-overflow.html
- flexbox_wrap-long.html
- flexbox_wrap-reverse.html
- flexbox_wrap.html
- flexible-order.html
- gap-001-lr.html
- gap-001-ltr.html
- gap-001-rl.html
- gap-002-lr.html
- gap-002-ltr.html
- gap-002-rl.html
- gap-002-rtl.html
- gap-003-lr.html
- gap-003-ltr.html
- gap-003-rl.html
- gap-004-lr.html
- gap-004-ltr.html
- gap-004-rl.html
- gap-005-lr.html
- gap-005-ltr.html
- gap-005-rl.html
- gap-005-rtl.html
- gap-006-lr.html
- gap-006-ltr.html
- gap-006-rl.html
- gap-008-ltr.html
- gap-009-ltr.html
- gap-010-ltr.html
- gap-011.html
- gap-012.html
- gap-013.html
- gap-014.html
- gap-015.html
- gap-016.html
- gap-019.html
- gap-020.html
- gap-021.html
- grandchild-span-height.html
- grid-flex-item-003.html
- grid-flex-item-004.html
- grid-flex-item-006.html
- grid-flex-item-007.html
- inline-flex-min-content-height.html
- intrinsic-size/col-flex-grow-negative-margin-001.html
- intrinsic-size/col-negative-margin-hidden-grid-item-001.html
- intrinsic-size/col-wrap-011.html
- intrinsic-size/col-wrap-012.html
- intrinsic-size/col-wrap-015.html
- intrinsic-size/col-wrap-016.html
- intrinsic-size/col-wrap-017.html
- intrinsic-size/row-001.html
- intrinsic-size/row-004.html
- intrinsic-size/row-006.html
- intrinsic-size/row-007.html
- intrinsic-size/row-009.html
- intrinsic-size/row-010.html
- item-with-max-height-and-scrollbar.html
- item-with-table-with-infinite-max-intrinsic-width.html
- justify-content_space-between-003.tentative.html
- layout-algorithm_algo-cross-line-001.html
- layout-algorithm_algo-cross-line-002.html
- min-size-auto-overflow-clip.html
- multiline-column-max-height.html
- multiline-last-baseline-item-count.html
- negative-margins-001.html
- order/order-abs-children-painting-order-different-container.html
- order/order-with-column-reverse.html
- order/order-with-row-reverse.html
- order-painting.html
- ortho-table-item-001.html
- overflow-area-003.html
- padding-overflow.html
- percentage-heights-002.html
- percentage-heights-004.html
- percentage-heights-005.html
- percentage-heights-006.html
- percentage-heights-007.html
- percentage-heights-008.html
- percentage-heights-009.html
- percentage-heights-010.html
- percentage-heights-016.html
- percentage-heights-017.html
- percentage-heights-018.html
- percentage-heights-019.html
- percentage-heights-021.html
- percentage-max-height-001.html
- percentage-max-height-004.html
- percentage-max-height-005.html
- percentage-padding-002.html
- percentage-padding-005.html
- percentage-size-quirks-003.html
- percentage-size-subitems-001.html
- position-fixed-001.html
- position-relative-percentage-top-003.html
- stretch-flex-item-checkbox-input.html
- stretch-flex-item-radio-input.html
- stretch-input-in-column.html
- stretch-obeys-min-max-001.html
- stretch-obeys-min-max-002.html
- stretch-obeys-min-max-003.html
- stretched-child-in-nested-flexbox-001.html
- stretched-child-in-nested-flexbox-002.html
- stretched-child-in-nested-flexbox-003.html
- stretching-orthogonal-flows.html
- synthesize-vrl-baseline.html
- table-as-flex-item-max-content.html
- table-as-item-auto-min-width.html
- table-as-item-fixed-min-width-2.html
- table-as-item-fixed-min-width.html
- table-as-item-inflexible-in-column-1.html
- table-as-item-inflexible-in-column-2.html
- table-as-item-inflexible-in-row-1.html
- table-as-item-inflexible-in-row-2.html
- table-as-item-large-intrinsic-size.html
- table-as-item-min-height-1.html
- table-as-item-narrow-content-2.html
- table-as-item-narrow-content.html
- table-as-item-percent-width-cell-001.html
- table-as-item-specified-height.html
- table-as-item-specified-width-vertical.html
- table-as-item-specified-width.html
- table-as-item-stretch-cross-size-3.html
- table-as-item-stretch-cross-size-4.html
- table-as-item-wide-content.html
- table-item-flex-percentage-min-width.html
- table-item-flex-percentage-width.html
- table-with-float-paint.html
- table-with-infinite-max-intrinsic-width.html
- text-overflow-on-flexbox-001.html
- webkit-box-vertical-writing-mode.html
- whitespace-in-flexitem-001.html
