# WPT `css/css-grid`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/css-grid`，用时 12.2 秒。

- 通过 **434 / 713**（60.9%）
- bug 类失败：**160**
- 跳过：no reftest 6

失败分类：bug 66，bug (flex/grid with vertical writing) 23，bug (open) 71，limit (not supported) 4，limit (placeholder boxes) 19，limit (replaced stretch) 1，limit (scrollable overflow) 6，not planned 82，step 2d-2b (vertical writing modes) 2，step 2e 5

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| abspos/descendant-static-position-001.html | bug (open) | out-of-flow children of a grid container (css-grid-1 9): the grid area as containing block and the static position inside it are done; this test differs for a reason that was not analysed one by one (6581 pixels differ (largest difference 255); first at (12,31): #008000, the reference has #ffffff) |
| abspos/descendant-static-position-002.html | bug | 15064 pixels differ (largest difference 255); first at (12,31): #008000, the reference has #ffffff |
| abspos/descendant-static-position-003.html | bug (open) | out-of-flow children of a grid container (css-grid-1 9): the grid area as containing block and the static position inside it are done; this test differs for a reason that was not analysed one by one (808 pixels differ (largest difference 255); first at (72,31): #008000, the reference has #ffffff) |
| abspos/descendant-static-position-004.html | bug | 15750 pixels differ (largest difference 255); first at (51,31): #ff0000, the reference has #ffffff |
| abspos/grid-abspos-staticpos-align-self-rtl-001.html | bug | 1872 pixels differ (largest difference 255); first at (16,12): #008080, the reference has #ffff00 |
| abspos/grid-abspos-staticpos-align-self-rtl-002.html | bug | 1872 pixels differ (largest difference 255); first at (16,12): #008080, the reference has #ffff00 |
| abspos/grid-abspos-staticpos-align-self-rtl-003.html | bug | 1872 pixels differ (largest difference 255); first at (11,10): #008080, the reference has #ffff00 |
| abspos/grid-abspos-staticpos-align-self-rtl-004.html | bug | 1872 pixels differ (largest difference 255); first at (11,10): #008080, the reference has #ffff00 |
| abspos/grid-abspos-staticpos-align-self-rtl-last-baseline-001.html | bug | 144 pixels differ (largest difference 255); first at (37,6): #ffffff, the reference has #008080 |
| abspos/grid-abspos-staticpos-align-self-rtl-last-baseline-002.html | bug | 144 pixels differ (largest difference 255); first at (37,6): #ffffff, the reference has #008080 |
| abspos/grid-abspos-staticpos-align-self-rtl-last-baseline-003.html | bug | 144 pixels differ (largest difference 255); first at (40,6): #ffffff, the reference has #008080 |
| abspos/grid-abspos-staticpos-align-self-rtl-last-baseline-004.html | bug | 144 pixels differ (largest difference 255); first at (40,6): #ffffff, the reference has #008080 |
| abspos/grid-abspos-staticpos-align-self-safe-001.html | bug | 2594 pixels differ (largest difference 255); first at (77,4): #008080, the reference has #ffffff |
| abspos/grid-abspos-staticpos-align-self-safe-outer-cb-001.tentative.html | bug | 2070 pixels differ (largest difference 255); first at (152,6): #ffffff, the reference has #008080 |
| abspos/grid-abspos-staticpos-align-self-safe-outer-cb-003.tentative.html | bug | 1110 pixels differ (largest difference 255); first at (192,6): #ffffff, the reference has #008080 |
| abspos/grid-abspos-staticpos-align-self-vertWM-001.html | bug | 2492 pixels differ (largest difference 255); first at (13,13): #008080, the reference has #ffff00 |
| abspos/grid-abspos-staticpos-align-self-vertWM-002.html | bug | 2448 pixels differ (largest difference 255); first at (13,13): #008080, the reference has #ffff00 |
| abspos/grid-abspos-staticpos-align-self-vertWM-003.html | bug | 2016 pixels differ (largest difference 255); first at (10,11): #008080, the reference has #ffff00 |
| abspos/grid-abspos-staticpos-align-self-vertWM-004.html | bug | 1888 pixels differ (largest difference 255); first at (10,11): #008080, the reference has #ffff00 |
| abspos/grid-abspos-staticpos-align-self-vertWM-last-baseline-001.html | bug | 192 pixels differ (largest difference 255); first at (52,5): #008080, the reference has #ffffff |
| abspos/grid-abspos-staticpos-align-self-vertWM-last-baseline-002.html | bug | 192 pixels differ (largest difference 255); first at (52,5): #008080, the reference has #ffffff |
| abspos/grid-abspos-staticpos-align-self-vertWM-last-baseline-003.html | bug | 144 pixels differ (largest difference 255); first at (49,7): #008080, the reference has #ffffff |
| abspos/grid-abspos-staticpos-align-self-vertWM-last-baseline-004.html | bug | 144 pixels differ (largest difference 255); first at (49,7): #008080, the reference has #ffffff |
| abspos/grid-abspos-staticpos-justify-self-rtl-001.html | bug | 480 pixels differ (largest difference 255); first at (18,14): #008080, the reference has #ffff00 |
| abspos/grid-abspos-staticpos-justify-self-rtl-002.html | bug | 800 pixels differ (largest difference 255); first at (18,14): #008080, the reference has #ffff00 |
| abspos/grid-abspos-staticpos-justify-self-rtl-003.html | bug | 480 pixels differ (largest difference 255); first at (10,11): #008080, the reference has #ffff00 |
| abspos/grid-abspos-staticpos-justify-self-rtl-004.html | bug | 800 pixels differ (largest difference 255); first at (10,11): #008080, the reference has #ffff00 |
| abspos/grid-abspos-staticpos-justify-self-vertWM-001.html | bug | 2440 pixels differ (largest difference 255); first at (22,12): #ffff00, the reference has #008080 |
| abspos/grid-abspos-staticpos-justify-self-vertWM-002.html | bug | 2440 pixels differ (largest difference 255); first at (22,12): #ffff00, the reference has #008080 |
| abspos/grid-abspos-staticpos-justify-self-vertWM-003.html | bug | 1936 pixels differ (largest difference 255); first at (11,10): #008080, the reference has #ffff00 |
| abspos/grid-abspos-staticpos-justify-self-vertWM-004.html | bug | 1936 pixels differ (largest difference 255); first at (11,10): #008080, the reference has #ffff00 |
| abspos/grid-abspos-staticpos-justify-self-vertWM-last-baseline-001.html | bug | 192 pixels differ (largest difference 255); first at (37,6): #ffffff, the reference has #008080 |
| abspos/grid-abspos-staticpos-justify-self-vertWM-last-baseline-002.html | bug | 192 pixels differ (largest difference 255); first at (37,6): #ffffff, the reference has #008080 |
| abspos/grid-abspos-staticpos-justify-self-vertWM-last-baseline-003.html | bug | 160 pixels differ (largest difference 255); first at (40,6): #ffffff, the reference has #008080 |
| abspos/grid-abspos-staticpos-justify-self-vertWM-last-baseline-004.html | bug | 160 pixels differ (largest difference 255); first at (40,6): #ffffff, the reference has #008080 |
| abspos/grid-paint-positioned-children-001.html | bug (open) | out-of-flow children of a grid container (css-grid-1 9): the grid area as containing block and the static position inside it are done; this test differs for a reason that was not analysed one by one (1000 pixels differ (largest difference 255); first at (10,114): #808080, the reference has #800080) |
| abspos/grid-positioned-children-writing-modes-001.html | bug | 8000 pixels differ (largest difference 128); first at (223,111): #008000, the reference has #808080 |
| abspos/grid-positioned-items-background-rtl-001.html | bug (open) | out-of-flow children of a grid container (css-grid-1 9): the grid area as containing block and the static position inside it are done; this test differs for a reason that was not analysed one by one (3000 pixels differ (largest difference 255); first at (458,179): #008000, the reference has #ffffff) |
| abspos/orthogonal-positioned-grid-items-005.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (32000 pixels differ (largest difference 255); first at (191,93): #ffffff, the reference has #000000) |
| abspos/orthogonal-positioned-grid-items-010.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (109000 pixels differ (largest difference 255); first at (55,93): #ff00ff, the reference has #ffffff) |
| abspos/orthogonal-positioned-grid-items-012.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (32000 pixels differ (largest difference 255); first at (186,93): #ffffff, the reference has #000000) |
| abspos/orthogonal-positioned-grid-items-014.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (1568 pixels differ (largest difference 255); first at (23,243): #ff00ff, the reference has #ffffff) |
| abspos/orthogonal-positioned-grid-items-015.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (4096 pixels differ (largest difference 255); first at (23,73): #000000, the reference has #ffffff) |
| abspos/orthogonal-positioned-grid-items-016.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (3008 pixels differ (largest difference 255); first at (23,223): #ff00ff, the reference has #ffffff) |
| abspos/orthogonal-positioned-grid-items-017.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (3008 pixels differ (largest difference 255); first at (33,223): #ff00ff, the reference has #ffffff) |
| abspos/positioned-grid-descendant-containing-block-009.html | bug | 4681 pixels differ (largest difference 255); first at (129,115): #0000ff, the reference has #ff0000 |
| abspos/positioned-grid-descendant-containing-block-010.html | bug | 39081 pixels differ (largest difference 255); first at (129,115): #0000ff, the reference has #ff0000 |
| abspos/positioned-grid-descendant-containing-block-013.html | bug | 39081 pixels differ (largest difference 255); first at (129,115): #0000ff, the reference has #ff0000 |
| abspos/positioned-grid-descendant-containing-block-014.html | bug (open) | out-of-flow children of a grid container (css-grid-1 9): the grid area as containing block and the static position inside it are done; this test differs for a reason that was not analysed one by one (10000 pixels differ (largest difference 255); first at (8,148): #008000, the reference has #ffffff) |
| abspos/positioned-grid-descendant-containing-block-015.html | bug (open) | out-of-flow children of a grid container (css-grid-1 9): the grid area as containing block and the static position inside it are done; this test differs for a reason that was not analysed one by one (10000 pixels differ (largest difference 255); first at (108,48): #008000, the reference has #ffffff) |
| abspos/positioned-grid-descendants-017.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (8125 pixels differ (largest difference 255); first at (8,48): #008000, the reference has #ff69b4) |
| abspos/positioned-grid-items-023.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (625 pixels differ (largest difference 255); first at (58,98): #008000, the reference has #ff0000) |
| abspos/positioned-grid-items-sizing-001.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (19600 pixels differ (largest difference 255); first at (70,210): #000000, the reference has #808080) |
| alignment/grid-align-baseline-005.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (1016 pixels differ (largest difference 118); first at (8,64): #767676, the reference has #008000) |
| alignment/grid-align-baseline-explicit-placement.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (2000 pixels differ (largest difference 255); first at (8,80): #008000, the reference has #ffffff) |
| alignment/grid-baseline-align-001.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (5272 pixels differ (largest difference 255); first at (13,71): #000000, the reference has #ffffff) |
| alignment/grid-baseline-align-cycles-001.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (25992 pixels differ (largest difference 255); first at (39,84): #ffffff, the reference has #c0c0c0) |
| alignment/grid-baseline-justify-001.html | bug | 30868 pixels differ (largest difference 255); first at (36,68): #ffffff, the reference has #c0c0c0 |
| alignment/grid-gutters-009.html | bug (open) | percentage `gap` whose basis is indefinite (shrink-to-fit inline grid, auto height): it is 0 while the tracks are sized and resolved against the used size afterwards (css-align-3 8.3); the order of the steps differs from the browsers (18180 pixels differ (largest difference 255); first at (8,170): #c0c0c0, the reference has #008000) |
| alignment/grid-gutters-010.html | bug (open) | percentage `gap` whose basis is indefinite (shrink-to-fit inline grid, auto height): it is 0 while the tracks are sized and resolved against the used size afterwards (css-align-3 8.3); the order of the steps differs from the browsers (18180 pixels differ (largest difference 255); first at (8,170): #c0c0c0, the reference has #008000) |
| alignment/grid-gutters-014.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (2520 pixels differ (largest difference 255); first at (28,113): #00ffff, the reference has #ffffff) |
| alignment/grid-gutters-015.html | bug (open) | percentage `gap` whose basis is indefinite (shrink-to-fit inline grid, auto height): it is 0 while the tracks are sized and resolved against the used size afterwards (css-align-3 8.3); the order of the steps differs from the browsers (2560 pixels differ (largest difference 255); first at (21,61): #008000, the reference has #ffffff) |
| alignment/grid-gutters-016.html | bug (open) | percentage `gap` whose basis is indefinite (shrink-to-fit inline grid, auto height): it is 0 while the tracks are sized and resolved against the used size afterwards (css-align-3 8.3); the order of the steps differs from the browsers (4480 pixels differ (largest difference 255); first at (21,61): #008000, the reference has #ffffff) |
| alignment/grid-inline-baseline.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (5632 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #000000) |
| alignment/grid-item-content-baseline-001.html | bug | 23607 pixels differ (largest difference 255); first at (5,5): #000000, the reference has #00ff00 |
| alignment/grid-item-content-baseline-002.html | bug | 114936 pixels differ (largest difference 255); first at (5,5): #000000, the reference has #00ff00 |
| alignment/grid-item-content-baseline-003.html | bug | 92645 pixels differ (largest difference 255); first at (161,1): #ffffff, the reference has #000000 |
| alignment/grid-item-content-baseline-004.html | bug | 98953 pixels differ (largest difference 255); first at (161,1): #ffffff, the reference has #000000 |
| alignment/grid-item-mixed-baseline-001.html | bug | 10605 pixels differ (largest difference 255); first at (36,4): #000000, the reference has #00ff00 |
| alignment/grid-item-mixed-baseline-002.html | bug | 10293 pixels differ (largest difference 255); first at (36,4): #000000, the reference has #00ff00 |
| alignment/grid-item-mixed-baseline-003.html | bug | 10293 pixels differ (largest difference 255); first at (36,4): #000000, the reference has #00ff00 |
| alignment/grid-item-mixed-baseline-004.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (4308 pixels differ (largest difference 255); first at (3,105): #000000, the reference has #ffffff) |
| alignment/grid-item-no-aspect-ratio-stretch-1.html | limit (replaced stretch) | an inline `<svg>` without viewBox is a replaced box of 300 x 150; a grid item that is replaced and has no aspect ratio should stretch with align/justify normal or stretch (css-grid-1 6.2), the grid code does not stretch replaced items; passed before 2e-1 only because `svg` was ignored (80000 pixels differ (largest difference 128); first at (318,18): #800080, the reference has #008000) |
| alignment/grid-item-self-baseline-001.html | bug | 86818 pixels differ (largest difference 255); first at (215,1): #ffffff, the reference has #000000 |
| alignment/grid-justify-baseline-001.html | bug | 7200 pixels differ (largest difference 255); first at (8,56): #808080, the reference has #000000 |
| alignment/grid-self-alignment-stretch-input-range.html | step 2e | uses scrolling, sticky or form controls (2e-2) (13512 pixels differ (largest difference 255); first at (70,9): #cfe5ff, the reference has #ffffff) |
| alignment/replaced-alignment-with-aspect-ratio-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (9961 pixels differ (largest difference 255); first at (8,64): #767676, the reference has #008000) |
| alignment/replaced-alignment-with-aspect-ratio-002.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (9961 pixels differ (largest difference 255); first at (8,64): #767676, the reference has #008000) |
| alignment/replaced-alignment-with-aspect-ratio-003.tentative.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (9931 pixels differ (largest difference 255); first at (8,64): #767676, the reference has #008000) |
| alignment/replaced-alignment-with-aspect-ratio-004.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (8750 pixels differ (largest difference 255); first at (33,64): #ff0000, the reference has #008000) |
| alignment/replaced-alignment-with-aspect-ratio-005.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (8750 pixels differ (largest difference 255); first at (33,64): #ff0000, the reference has #008000) |
| alignment/replaced-alignment-with-aspect-ratio-006.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (7500 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| alignment/replaced-alignment-with-aspect-ratio-007.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (7500 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| alignment/replaced-alignment-with-aspect-ratio-008.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (7500 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| alignment/replaced-alignment-with-aspect-ratio-009.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (7500 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| alignment/self-baseline/grid-self-baseline-001.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (233905 pixels differ (largest difference 255); first at (17,63): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-002-b.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (37052 pixels differ (largest difference 255); first at (17,79): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-002.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (173112 pixels differ (largest difference 255); first at (180,69): #808080, the reference has #ffffff) |
| alignment/self-baseline/grid-self-baseline-003.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (68547 pixels differ (largest difference 255); first at (17,63): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-004.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (170433 pixels differ (largest difference 255); first at (17,63): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-005.html | bug | 174283 pixels differ (largest difference 255); first at (445,53): #ffffff, the reference has #808080 |
| alignment/self-baseline/grid-self-baseline-006.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (163184 pixels differ (largest difference 255); first at (5,53): #808080, the reference has #ffffff) |
| alignment/self-baseline/grid-self-baseline-007.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (184229 pixels differ (largest difference 255); first at (450,53): #ffffff, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-008.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (48064 pixels differ (largest difference 255); first at (8,8): #767676, the reference has #008000) |
| alignment/self-baseline/grid-self-baseline-changes-grid-area-size-001.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (400 pixels differ (largest difference 255); first at (28,84): #ff0000, the reference has #008000) |
| alignment/self-baseline/grid-self-baseline-changes-grid-area-size-002.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (350 pixels differ (largest difference 255); first at (58,81): #ff0000, the reference has #008000) |
| alignment/self-baseline/grid-self-baseline-changes-grid-area-size-003.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (520 pixels differ (largest difference 255); first at (58,88): #ff0000, the reference has #008000) |
| alignment/self-baseline/grid-self-baseline-changes-grid-area-size-004.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (400 pixels differ (largest difference 255); first at (28,84): #ff0000, the reference has #008000) |
| alignment/self-baseline/grid-self-baseline-changes-grid-area-size-005.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (560 pixels differ (largest difference 255); first at (28,84): #ff0000, the reference has #008000) |
| alignment/self-baseline/grid-self-baseline-changes-grid-area-size-006.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (520 pixels differ (largest difference 255); first at (58,88): #ff0000, the reference has #008000) |
| alignment/self-baseline/grid-self-baseline-changes-grid-area-size-007.html | bug | 500 pixels differ (largest difference 255); first at (38,64): #ff0000, the reference has #008000 |
| alignment/self-baseline/grid-self-baseline-changes-grid-area-size-008.html | bug | 850 pixels differ (largest difference 255); first at (31,64): #ff0000, the reference has #008000 |
| alignment/self-baseline/grid-self-baseline-changes-grid-area-size-009.html | bug | 400 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000 |
| alignment/self-baseline/grid-self-baseline-changes-grid-area-size-010.html | bug | 920 pixels differ (largest difference 255); first at (13,64): #ff0000, the reference has #008000 |
| alignment/self-baseline/grid-self-baseline-changes-grid-area-size-011.html | bug | 930 pixels differ (largest difference 255); first at (13,64): #ff0000, the reference has #008000 |
| alignment/self-baseline/grid-self-baseline-changes-grid-area-size-012.html | bug | 400 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000 |
| alignment/self-baseline/grid-self-baseline-horiz-001.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (16064 pixels differ (largest difference 255); first at (17,63): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-horiz-002.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (110535 pixels differ (largest difference 255); first at (17,63): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-horiz-003.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (54366 pixels differ (largest difference 128); first at (17,63): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-horiz-004.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (16061 pixels differ (largest difference 255); first at (17,63): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-horiz-005.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (110881 pixels differ (largest difference 255); first at (39,53): #808080, the reference has #000000) |
| alignment/self-baseline/grid-self-baseline-horiz-006.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (109327 pixels differ (largest difference 255); first at (43,53): #ffffff, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-horiz-007.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (36784 pixels differ (largest difference 255); first at (17,79): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-vertical-lr-001.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (67315 pixels differ (largest difference 255); first at (107,53): #808080, the reference has #ffffff) |
| alignment/self-baseline/grid-self-baseline-vertical-lr-002.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (107968 pixels differ (largest difference 255); first at (39,53): #808080, the reference has #000000) |
| alignment/self-baseline/grid-self-baseline-vertical-lr-003.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (34832 pixels differ (largest difference 255); first at (17,63): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-vertical-lr-004.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (42482 pixels differ (largest difference 255); first at (17,63): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-vertical-lr-005.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (170171 pixels differ (largest difference 255); first at (17,63): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-vertical-lr-006.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (176220 pixels differ (largest difference 255); first at (17,63): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-vertical-lr-007.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (29976 pixels differ (largest difference 255); first at (17,79): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-vertical-rl-001.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (67427 pixels differ (largest difference 255); first at (107,53): #808080, the reference has #ffffff) |
| alignment/self-baseline/grid-self-baseline-vertical-rl-002.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (127973 pixels differ (largest difference 255); first at (46,53): #808080, the reference has #000000) |
| alignment/self-baseline/grid-self-baseline-vertical-rl-003.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (35840 pixels differ (largest difference 255); first at (17,63): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-vertical-rl-004.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (43826 pixels differ (largest difference 255); first at (17,63): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-vertical-rl-005.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (172175 pixels differ (largest difference 255); first at (17,63): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-vertical-rl-006.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (176404 pixels differ (largest difference 255); first at (17,63): #000000, the reference has #808080) |
| alignment/self-baseline/grid-self-baseline-vertical-rl-007.html | bug (open) | baseline alignment of grid items (css-grid-1 6.3, 11.5 "shim baseline-aligned items", css-align-3 9) is not done: `baseline` lays out as `start`, there are no baseline-sharing groups (34122 pixels differ (largest difference 255); first at (0,79): #ffffff, the reference has #000000) |
| firefox-bug-1881495.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (8280 pixels differ (largest difference 255); first at (42,110): #808080, the reference has #000000) |
| grid-container-baseline-synthesized-001.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (2664 pixels differ (largest difference 255); first at (54,92): #00ffff, the reference has #000000) |
| grid-container-baseline-synthesized-002.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (3330 pixels differ (largest difference 255); first at (54,8): #00ffff, the reference has #000000) |
| grid-container-baseline-synthesized-003.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (2664 pixels differ (largest difference 255); first at (54,92): #00ffff, the reference has #000000) |
| grid-container-baseline-synthesized-004.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (3330 pixels differ (largest difference 255); first at (54,8): #00ffff, the reference has #000000) |
| grid-definition/fr-unit-with-percentage.html | bug (open) | not the grid (its boxes are right in the layout dump): the helper table with height:100% shares its extra height among the rows in proportion to their heights (table.rs), the browsers give it to the auto-height rows first (css-tables-3, extra height distribution) (14340 pixels differ (largest difference 255); first at (8,94): #000000, the reference has #ffffff) |
| grid-definition/fr-unit.html | bug (open) | not the grid (its boxes are right in the layout dump): the helper table with height:100% shares its extra height among the rows in proportion to their heights (table.rs), the browsers give it to the auto-height rows first (css-tables-3, extra height distribution) (14340 pixels differ (largest difference 255); first at (8,94): #000000, the reference has #ffffff) |
| grid-definition/grid-auto-repeat-aspect-ratio-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (7000 pixels differ (largest difference 255); first at (58,48): #ffffff, the reference has #008000) |
| grid-definition/grid-auto-repeat-aspect-ratio-002.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (5000 pixels differ (largest difference 255); first at (58,48): #ffffff, the reference has #008000) |
| grid-definition/grid-item-grid-container-percentage-rows-002.html | bug (open) | percentage row tracks / heights against an indefinite container height: treated as `auto` and not resolved again after the rows are sized (css-grid-1 7.2.1, 11.1) (1260 pixels differ (largest difference 128); first at (273,303): #00ffff, the reference has #808080) |
| grid-definition/grid-template-columns-fit-content-001.html | bug (open) | not analysed one by one: a css-grid-1 test whose intrinsic sizing or placement still differs from the reference (fit-content(), spanning flexible tracks, max-content sizing with percentages, floats beside a grid) (48263 pixels differ (largest difference 255); first at (203,226): #800080, the reference has #ffa500) |
| grid-definition/grid-template-rows-fit-content-001.html | bug | 20926 pixels differ (largest difference 255); first at (140,64): #808080, the reference has #000000 |
| grid-flex-spanning-items-001.html | bug (open) | not analysed one by one: a css-grid-1 test whose intrinsic sizing or placement still differs from the reference (fit-content(), spanning flexible tracks, max-content sizing with percentages, floats beside a grid) (5900 pixels differ (largest difference 255); first at (58,8): #ff00ff, the reference has #ffffff) |
| grid-in-table-cell-with-img.html | bug (open) | percentage heights of replaced items in a row of the grid (cyclic percentage, css-sizing-4 5.2.2): the width an image contributes to its column depends on the row size, which is not known yet; the columns are not sized again after the rows (css-grid-1 11.1) (10000 pixels differ (largest difference 255); first at (108,48): #008000, the reference has #ffffff) |
| grid-item-percentage-quirk-001.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (/quirks/reference/green-100px-square-no-red.html answered HTTP 404) |
| grid-item-percentage-quirk-002.html | bug (open) | not analysed one by one: a css-grid-1 test whose intrinsic sizing or placement still differs from the reference (fit-content(), spanning flexible tracks, max-content sizing with percentages, floats beside a grid) (/quirks/reference/green-100px-square-no-red.html answered HTTP 404) |
| grid-items/aspect-ratio-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| grid-items/aspect-ratio-002.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| grid-items/aspect-ratio-003.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| grid-items/aspect-ratio-004.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (5000 pixels differ (largest difference 255); first at (58,48): #ffffff, the reference has #008000) |
| grid-items/aspect-ratio-005.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| grid-items/grid-img-item-percent-max-height-001.html | bug (open) | percentage heights of replaced items in a row of the grid (cyclic percentage, css-sizing-4 5.2.2): the width an image contributes to its column depends on the row size, which is not known yet; the columns are not sized again after the rows (css-grid-1 11.1) (10000 pixels differ (largest difference 255); first at (108,48): #008000, the reference has #ffffff) |
| grid-items/grid-item-aspect-ratio-percent-width-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (40880 pixels differ (largest difference 255); first at (31,11): #000000, the reference has #00ffff) |
| grid-items/grid-item-direction-rtl-001.html | bug | 5000 pixels differ (largest difference 255); first at (8,64): #008000, the reference has #ffffff |
| grid-items/grid-item-inline-contribution-001.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (8131 pixels differ (largest difference 255); first at (28,64): #ffffff, the reference has #008000) |
| grid-items/grid-item-inline-contribution-002.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (9061 pixels differ (largest difference 255); first at (18,64): #ffffff, the reference has #008000) |
| grid-items/grid-item-inline-contribution-003.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (9061 pixels differ (largest difference 255); first at (18,64): #ffffff, the reference has #008000) |
| grid-items/grid-item-inline-contribution-004.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (4526 pixels differ (largest difference 255); first at (63,64): #ffffff, the reference has #008000) |
| grid-items/grid-item-inline-contribution-005.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (9061 pixels differ (largest difference 255); first at (18,64): #ffffff, the reference has #008000) |
| grid-items/grid-item-margins-and-writing-modes-001.html | step 2d-2b (vertical writing modes) | used to pass only because logical margins were mapped as in ltr in every direction; with `direction: rtl` now flipping them, the cases with writing-mode: vertical-* (2d-2b) differ from the reference (4930 pixels differ (largest difference 128); first at (25,100): #008000, the reference has #808080) |
| grid-items/grid-item-minmax-img-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (2600 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| grid-items/grid-item-minmax-img-002.html | bug | 1325 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #daa520 |
| grid-items/grid-item-paddings-and-writing-modes-001.html | step 2d-2b (vertical writing modes) | same as grid-item-margins-and-writing-modes-001 (6400 pixels differ (largest difference 128); first at (24,112): #008000, the reference has #808080) |
| grid-items/grid-item-percentage-sizes-001.html | bug | 18459 pixels differ (largest difference 255); first at (199,2): #00ffff, the reference has #ffffff |
| grid-items/grid-item-percentage-sizes-002.html | bug | 18270 pixels differ (largest difference 255); first at (201,2): #00ffff, the reference has #ffffff |
| grid-items/grid-item-percentage-sizes-003.html | bug | 20537 pixels differ (largest difference 255); first at (82,2): #00ffff, the reference has #ffffff |
| grid-items/grid-item-text-background-not-interleaved-001.html | bug (open) | grid items now paint atomically like inline blocks (css-grid-1 9, CSS2 E.2 step 7) and the other tests of this family pass; this one still differs (items with a `z-index` or overlapping lines of text), not analysed one by one (224 pixels differ (largest difference 128); first at (108,112): #000000, the reference has #808080) |
| grid-items/grid-items-relative-positioned-containing-block-004.html | bug (open) | out-of-flow children of a grid container (css-grid-1 9): the grid area as containing block and the static position inside it are done; this test differs for a reason that was not analysed one by one (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| grid-items/grid-items-relative-positioned-containing-block-005.html | bug (open) | out-of-flow children of a grid container (css-grid-1 9): the grid area as containing block and the static position inside it are done; this test differs for a reason that was not analysed one by one (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| grid-items/grid-items-relative-positioned-containing-block-006.html | bug (open) | out-of-flow children of a grid container (css-grid-1 9): the grid area as containing block and the static position inside it are done; this test differs for a reason that was not analysed one by one (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| grid-items/grid-items-sizing-alignment-001.html | step 2e | uses scrolling, sticky or form controls (2e-2) (42176 pixels differ (largest difference 255); first at (604,64): #7b7b7b, the reference has #767676) |
| grid-items/grid-layout-z-order-a.html | bug (open) | grid items now paint atomically like inline blocks (css-grid-1 9, CSS2 E.2 step 7) and the other tests of this family pass; this one still differs (items with a `z-index` or overlapping lines of text), not analysed one by one (224 pixels differ (largest difference 255); first at (100,60): #ffffff, the reference has #ffff00) |
| grid-items/grid-layout-z-order-b.html | bug (open) | grid items now paint atomically like inline blocks (css-grid-1 9, CSS2 E.2 step 7) and the other tests of this family pass; this one still differs (items with a `z-index` or overlapping lines of text), not analysed one by one (224 pixels differ (largest difference 255); first at (100,60): #ffffff, the reference has #ffff00) |
| grid-items/grid-size-with-orthogonal-child-001.html | bug | 10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000 |
| grid-items/grid-size-with-orthogonal-child-002.html | bug | 10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000 |
| grid-items/percentage-block-size-transfer-through-item-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (1916 pixels differ (largest difference 118); first at (8,48): #767676, the reference has #008000) |
| grid-items/percentage-size-indefinite-replaced.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (1916 pixels differ (largest difference 118); first at (8,48): #767676, the reference has #008000) |
| grid-items/percentage-size-replaced-subitems-001.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (76482 pixels differ (largest difference 255); first at (662,96): #ffffff, the reference has #ff00ff) |
| grid-items/percentage-size-subitems-001.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (76632 pixels differ (largest difference 255); first at (662,96): #ffffff, the reference has #ff00ff) |
| grid-items/replaced-element-011.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (9296 pixels differ (largest difference 255); first at (8,48): #767676, the reference has #008000) |
| grid-items/replaced-element-012.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (9296 pixels differ (largest difference 255); first at (8,48): #767676, the reference has #008000) |
| grid-items/replaced-element-013.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (8376 pixels differ (largest difference 255); first at (8,48): #767676, the reference has #008000) |
| grid-items/replaced-element-014.html | bug (open) | not analysed one by one: a css-grid-1 test whose intrinsic sizing or placement still differs from the reference (fit-content(), spanning flexible tracks, max-content sizing with percentages, floats beside a grid) (30000 pixels differ (largest difference 255); first at (108,48): #008000, the reference has #ffffff) |
| grid-items/replaced-element-016.tentative.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (1916 pixels differ (largest difference 118); first at (8,48): #767676, the reference has #008000) |
| grid-items/replaced-element-017.tentative.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (11834 pixels differ (largest difference 255); first at (8,48): #767676, the reference has #008000) |
| grid-model/display-grid.html | bug (open) | not the grid (its boxes are right in the layout dump): the helper table with height:100% shares its extra height among the rows in proportion to their heights (table.rs), the browsers give it to the auto-height rows first (css-tables-3, extra height distribution) (16560 pixels differ (largest difference 205); first at (8,99): #008000, the reference has #000000) |
| grid-model/display-inline-grid.html | bug (open) | not the grid (its boxes are right in the layout dump): the helper table with height:100% shares its extra height among the rows in proportion to their heights (table.rs), the browsers give it to the auto-height rows first (css-tables-3, extra height distribution) (16560 pixels differ (largest difference 205); first at (8,99): #008000, the reference has #000000) |
| grid-model/grid-areas-overflowing-grid-container-001.html | limit (scrollable overflow) | the tracks of an empty grid are not part of the scrollable overflow of the grid container (no bars) (700 pixels differ (largest difference 10); first at (100,64): #808080, the reference has #8a8a8a) |
| grid-model/grid-areas-overflowing-grid-container-002.html | limit (scrollable overflow) | same as grid-areas-overflowing-grid-container-001 (700 pixels differ (largest difference 10); first at (100,64): #808080, the reference has #8a8a8a) |
| grid-model/grid-areas-overflowing-grid-container-003.html | limit (scrollable overflow) | same as grid-areas-overflowing-grid-container-001 (700 pixels differ (largest difference 10); first at (100,64): #808080, the reference has #8a8a8a) |
| grid-model/grid-areas-overflowing-grid-container-009.html | limit (scrollable overflow) | same as grid-areas-overflowing-grid-container-001 (700 pixels differ (largest difference 10); first at (100,64): #808080, the reference has #8a8a8a) |
| grid-model/grid-container-scrollbar-vertical-lr-001.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (134700 pixels differ (largest difference 255); first at (28,152): #808080, the reference has #ffffff) |
| grid-model/grid-container-scrollbar-vertical-rl-001.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (136800 pixels differ (largest difference 255); first at (28,152): #808080, the reference has #ffffff) |
| grid-model/grid-first-letter-003.html | limit (not supported) | ::first-letter and ::first-line are not supported (not planned in layer 2) (256 pixels differ (largest difference 128); first at (8,24): #000000, the reference has #008000) |
| grid-model/grid-first-line-003.html | limit (not supported) | ::first-letter and ::first-line are not supported (not planned in layer 2) (16640 pixels differ (largest difference 255); first at (8,24): #ff0000, the reference has #008000) |
| grid-model/grid-floats-no-intrude-002.html | bug (open) | not analysed one by one: a css-grid-1 test whose intrinsic sizing or placement still differs from the reference (fit-content(), spanning flexible tracks, max-content sizing with percentages, floats beside a grid) (34560 pixels differ (largest difference 255); first at (88,88): #0000ff, the reference has #00ff00) |
| grid-model/grid-inline-first-letter-003.html | limit (not supported) | ::first-letter and ::first-line are not supported (not planned in layer 2) (256 pixels differ (largest difference 128); first at (8,24): #000000, the reference has #008000) |
| grid-model/grid-inline-first-line-003.html | limit (not supported) | ::first-letter and ::first-line are not supported (not planned in layer 2) (16640 pixels differ (largest difference 255); first at (8,24): #ff0000, the reference has #008000) |
| grid-model/grid-inline-floats-no-intrude-001.html | bug (open) | not analysed one by one: a css-grid-1 test whose intrinsic sizing or placement still differs from the reference (fit-content(), spanning flexible tracks, max-content sizing with percentages, floats beside a grid) (5000 pixels differ (largest difference 255); first at (108,64): #ff0000, the reference has #ffffff) |
| grid-model/grid-max-content-size-with-max-content-item.html | bug (open) | not analysed one by one: a css-grid-1 test whose intrinsic sizing or placement still differs from the reference (fit-content(), spanning flexible tracks, max-content sizing with percentages, floats beside a grid) (7200 pixels differ (largest difference 255); first at (28,48): #ffffff, the reference has #008000) |
| grid-model/grid-overflow-padding-001.html | limit (scrollable overflow) | an empty grid whose track plus end padding overflows: the tracks are not part of the scrollable overflow (700 pixels differ (largest difference 10); first at (100,64): #808080, the reference has #8a8a8a) |
| grid-model/grid-overflow-padding-002.html | limit (scrollable overflow) | same as grid-overflow-padding-001 (700 pixels differ (largest difference 10); first at (100,64): #808080, the reference has #8a8a8a) |
| grid-with-aspect-ratio-uses-content-box-height-for-track-sizing.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (4000 pixels differ (largest difference 255); first at (8,108): #ffffff, the reference has #008000) |
| grid-with-orthogonal-child-within-flexbox.html | bug | 10000 pixels differ (largest difference 255); first at (8,48): #000000, the reference has #008000 |
| layout-algorithm/flex-sizing-rows-indefinite-height-002.html | bug (open) | percentage row tracks / heights against an indefinite container height: treated as `auto` and not resolved again after the rows are sized (css-grid-1 7.2.1, 11.1) (1600 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| layout-algorithm/flex-sizing-rows-indefinite-height.html | bug | 10000 pixels differ (largest difference 255); first at (8,164): #ff0000, the reference has #ffffff |
| layout-algorithm/grid-as-flex-item-should-not-shrink-to-fit-003.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| layout-algorithm/grid-as-flex-item-should-not-shrink-to-fit-004.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| layout-algorithm/grid-fit-content-width-percent-height-aspect-ratio-001.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (8131 pixels differ (largest difference 255); first at (28,48): #ffffff, the reference has #008000) |
| layout-algorithm/grid-float-intrinsic-width-percent-height-aspect-ratio-001.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (8131 pixels differ (largest difference 255); first at (28,48): #ffffff, the reference has #008000) |
| layout-algorithm/grid-inline-grid-intrinsic-width-percent-height-aspect-ratio-001.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (8131 pixels differ (largest difference 255); first at (28,48): #ffffff, the reference has #008000) |
| layout-algorithm/grid-item-margin-auto-columns-rows-vertical-rtl-001.html | bug | 42800 pixels differ (largest difference 255); first at (168,56): #00ff00, the reference has #ffa500 |
| layout-algorithm/grid-max-content-width-percent-height-aspect-ratio-001.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (8131 pixels differ (largest difference 255); first at (28,48): #ffffff, the reference has #008000) |
| layout-algorithm/grid-min-content-width-percent-height-aspect-ratio-001.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (8131 pixels differ (largest difference 255); first at (28,48): #ffffff, the reference has #008000) |
| layout-algorithm/grid-percent-rows-filled-shrinkwrap-001.html | bug (open) | percentage row tracks / heights against an indefinite container height: treated as `auto` and not resolved again after the rows are sized (css-grid-1 7.2.1, 11.1) (1600 pixels differ (largest difference 255); first at (31,91): #ffffff, the reference has #0000ff) |
| layout-algorithm/grid-percent-rows-spanned-shrinkwrap-001.html | bug (open) | percentage row tracks / heights against an indefinite container height: treated as `auto` and not resolved again after the rows are sized (css-grid-1 7.2.1, 11.1) (800 pixels differ (largest difference 255); first at (31,91): #ffffff, the reference has #0000ff) |
| layout-algorithm/grid-template-flexible-rerun-track-sizing.html | bug | 10000 pixels differ (largest difference 255); first at (8,164): #ff0000, the reference has #ffffff |
| layout-algorithm/grid-track-ignores-max-size-002.html | limit (placeholder boxes) | canvas, marquee, iframe, video and object are placeholder boxes with their own sizes (their percentage height / aspect ratio behaviour is the browsers') (1916 pixels differ (largest difference 118); first at (8,48): #767676, the reference has #008000) |
| stretch-grid-item-checkbox-input.html | step 2e | uses scrolling, sticky or form controls (2e-2) (204 pixels differ (largest difference 137); first at (19,8): #828282, the reference has #767676) |
| stretch-grid-item-radio-input.html | step 2e | uses scrolling, sticky or form controls (2e-2) (414 pixels differ (largest difference 135); first at (11,8): #dcdcdc, the reference has #ffffff) |
| stretch-grid-item-text-input-overflow.html | step 2e | uses scrolling, sticky or form controls (2e-2) (2488 pixels differ (largest difference 255); first at (91,13): #171717, the reference has #000000) |
| subgrid/abs-pos-002.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (16674 pixels differ (largest difference 255); first at (36,17): #000000, the reference has #ffff00) |
| subgrid/auto-track-sizing-004.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (10000 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| subgrid/baseline-001.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (540 pixels differ (largest difference 255); first at (1,21): #ffffff, the reference has #000000) |
| subgrid/contribution-size-flex-tracks-001.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (7500 pixels differ (largest difference 255); first at (58,64): #ffffff, the reference has #008000) |
| subgrid/grid-gap-001.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (91584 pixels differ (largest difference 187); first at (84,0): #ffffff, the reference has #444444) |
| subgrid/grid-gap-002.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (240 pixels differ (largest difference 230); first at (11,59): #add8e6, the reference has #ffff00) |
| subgrid/grid-gap-003.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (5316 pixels differ (largest difference 255); first at (31,1): #ffffff, the reference has #808080) |
| subgrid/grid-gap-004.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (19857 pixels differ (largest difference 255); first at (154,0): #ffffff, the reference has #000000) |
| subgrid/grid-gap-005.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (12400 pixels differ (largest difference 255); first at (141,1): #808080, the reference has #ffffff) |
| subgrid/grid-gap-006.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (13000 pixels differ (largest difference 255); first at (341,201): #808080, the reference has #ffffff) |
| subgrid/grid-gap-007.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (44008 pixels differ (largest difference 255); first at (126,0): #ffffff, the reference has #000000) |
| subgrid/grid-gap-008.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (18450 pixels differ (largest difference 255); first at (3,3): #000000, the reference has #d3d3d3) |
| subgrid/grid-gap-009.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (111902 pixels differ (largest difference 255); first at (3,3): #000000, the reference has #d3d3d3) |
| subgrid/grid-gap-010.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (10000 pixels differ (largest difference 255); first at (58,8): #ffffff, the reference has #0000ff) |
| subgrid/grid-gap-011.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (25000 pixels differ (largest difference 255); first at (158,8): #0000ff, the reference has #ffffff) |
| subgrid/grid-gap-012.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (20000 pixels differ (largest difference 255); first at (233,8): #ffffff, the reference has #0000ff) |
| subgrid/grid-gap-larger-001.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (144544 pixels differ (largest difference 255); first at (240,76): #ffff00, the reference has #444444) |
| subgrid/grid-gap-larger-002.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (10964 pixels differ (largest difference 255); first at (3,19): #ffff00, the reference has #808080) |
| subgrid/grid-gap-normal-001.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (77884 pixels differ (largest difference 187); first at (260,130): #cccccc, the reference has #444444) |
| subgrid/grid-gap-smaller-001.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (131624 pixels differ (largest difference 255); first at (280,96): #ffff00, the reference has #800080) |
| subgrid/independent-formatting-context-fieldset.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (10000 pixels differ (largest difference 255); first at (0,0): #ff0000, the reference has #008000) |
| subgrid/independent-formatting-context.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (15000 pixels differ (largest difference 255); first at (136,120): #ff0000, the reference has #008000) |
| subgrid/item-percentage-height-001.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (1500 pixels differ (largest difference 83); first at (21,92): #d3d3d3, the reference has #808080) |
| subgrid/line-names-006.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (8724 pixels differ (largest difference 255); first at (107,12): #808080, the reference has #000000) |
| subgrid/line-names-008.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (1567 pixels differ (largest difference 255); first at (52,16): #808080, the reference has #000000) |
| subgrid/line-names-009.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (324 pixels differ (largest difference 192); first at (558,60): #000000, the reference has #808080) |
| subgrid/line-names-010.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (11784 pixels differ (largest difference 255); first at (466,36): #000000, the reference has #808080) |
| subgrid/line-names-011.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (432 pixels differ (largest difference 192); first at (151,36): #000000, the reference has #808080) |
| subgrid/line-names-012.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (17755 pixels differ (largest difference 192); first at (85,12): #808080, the reference has #000000) |
| subgrid/orthogonal-writing-mode-001.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (1400 pixels differ (largest difference 255); first at (9,191): #ffffff, the reference has #000000) |
| subgrid/orthogonal-writing-mode-002.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (40793 pixels differ (largest difference 255); first at (19,9): #ffffff, the reference has #808080) |
| subgrid/orthogonal-writing-mode-003.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (16882 pixels differ (largest difference 255); first at (9,23): #000000, the reference has #ffc0cb) |
| subgrid/orthogonal-writing-mode-004.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (25403 pixels differ (largest difference 255); first at (1,16): #ffc0cb, the reference has #000000) |
| subgrid/overflow-hidden-does-not-prohibit-subgrid.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (5000 pixels differ (largest difference 255); first at (8,98): #ffffff, the reference has #008000) |
| subgrid/placement-implicit-001.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (5808 pixels differ (largest difference 255); first at (204,0): #000000, the reference has #ffffff) |
| subgrid/repeat-auto-fill-002.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (38544 pixels differ (largest difference 255); first at (76,1): #808080, the reference has #ffc0cb) |
| subgrid/repeat-auto-fill-003.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (140296 pixels differ (largest difference 255); first at (1,1): #000000, the reference has #ffffff) |
| subgrid/repeat-auto-fill-004.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (38253 pixels differ (largest difference 255); first at (76,1): #808080, the reference has #ffc0cb) |
| subgrid/repeat-auto-fill-005.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (9740 pixels differ (largest difference 128); first at (60,0): #000000, the reference has #808080) |
| subgrid/row-subgrid-intrinsic-size-transfer-001.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| subgrid/row-subgrid-intrinsic-size-transfer-002.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| subgrid/row-subgrid-intrinsic-size-transfer-003.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (10000 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| subgrid/scrollbar-gutter-001.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (7500 pixels differ (largest difference 255); first at (114,101): #008000, the reference has #ffffff) |
| subgrid/scrollbar-gutter-002.html | bug (flex/grid with vertical writing) | flex and grid containers (and tables) are laid out horizontally inside and around vertical text (decisions.md 2d-2b): their items with a vertical writing mode are orthogonal frames the flex/grid algorithms do not size by the frame's block axis. Before 2d-2b this test passed because writing-mode was ignored. (7500 pixels differ (largest difference 255); first at (114,101): #008000, the reference has #ffffff) |
| subgrid/standalone-axis-size-005.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (10000 pixels differ (largest difference 255); first at (108,64): #ff0000, the reference has #ffffff) |
| subgrid/standalone-axis-size-007.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (10000 pixels differ (largest difference 255); first at (108,64): #008000, the reference has #ffffff) |
| subgrid/standalone-axis-size-010.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| subgrid/standalone-axis-size-011.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| subgrid/standalone-axis-size-012.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| subgrid/standalone-axis-size-013.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| subgrid/subgrid-baseline-001.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (540 pixels differ (largest difference 255); first at (1,21): #ffffff, the reference has #000000) |
| subgrid/subgrid-baseline-002.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (8364 pixels differ (largest difference 255); first at (1,21): #ffffff, the reference has #000000) |
| subgrid/subgrid-baseline-003.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (384 pixels differ (largest difference 255); first at (0,0): #ffffff, the reference has #000000) |
| subgrid/subgrid-baseline-004.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (384 pixels differ (largest difference 255); first at (0,0): #ffffff, the reference has #000000) |
| subgrid/subgrid-button.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (44 pixels differ (largest difference 204); first at (8,8): #cccccc, the reference has #000000) |
| subgrid/subgrid-item-block-size-001.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (78224 pixels differ (largest difference 255); first at (214,51): #ffffff, the reference has #000000) |
| subgrid/subgrid-item-with-margin-left-auto.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (5760 pixels differ (largest difference 255); first at (728,16): #000000, the reference has #ffffff) |
| subgrid/subgrid-no-items-on-edges-002.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (11200 pixels differ (largest difference 255); first at (38,64): #ff0000, the reference has #008000) |
| subgrid/subgrid-stretch.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (79702 pixels differ (largest difference 255); first at (232,0): #ffffff, the reference has #0000ff) |
| subgrid/writing-directions-001.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (1260 pixels differ (largest difference 192); first at (99,12): #000000, the reference has #808080) |
| subgrid/writing-directions-003.html | not planned | uses subgrid and masonry (grid-lanes) are not planned (2790 pixels differ (largest difference 255); first at (9,9): #000000, the reference has #808080) |

## 通过

- abspos/grid-abspos-staticpos-align-items-center-large-border-padding.html
- abspos/grid-abspos-staticpos-align-items-center.html
- abspos/grid-abspos-staticpos-align-items-end-large-border-padding.html
- abspos/grid-abspos-staticpos-align-items-end.html
- abspos/grid-abspos-staticpos-align-items-flex-end-large-border-padding.html
- abspos/grid-abspos-staticpos-align-items-flex-end.html
- abspos/grid-abspos-staticpos-align-items-self-end-large-border-padding.html
- abspos/grid-abspos-staticpos-align-items-self-end.html
- abspos/grid-abspos-staticpos-align-self-001.html
- abspos/grid-abspos-staticpos-align-self-002.html
- abspos/grid-abspos-staticpos-align-self-center-large-border-padding.html
- abspos/grid-abspos-staticpos-align-self-center.html
- abspos/grid-abspos-staticpos-align-self-end-large-border-padding.html
- abspos/grid-abspos-staticpos-align-self-end.html
- abspos/grid-abspos-staticpos-align-self-flex-end-large-border-padding.html
- abspos/grid-abspos-staticpos-align-self-flex-end.html
- abspos/grid-abspos-staticpos-align-self-img-001.html
- abspos/grid-abspos-staticpos-align-self-img-002.html
- abspos/grid-abspos-staticpos-align-self-img-last-baseline-001.html
- abspos/grid-abspos-staticpos-align-self-img-last-baseline-002.html
- abspos/grid-abspos-staticpos-align-self-last-baseline-001.html
- abspos/grid-abspos-staticpos-align-self-last-baseline-002.html
- abspos/grid-abspos-staticpos-align-self-safe-outer-cb-002.tentative.html
- abspos/grid-abspos-staticpos-align-self-self-end-large-border-padding.html
- abspos/grid-abspos-staticpos-align-self-self-end.html
- abspos/grid-abspos-staticpos-justify-self-001.html
- abspos/grid-abspos-staticpos-justify-self-002.html
- abspos/grid-abspos-staticpos-justify-self-img-001.html
- abspos/grid-abspos-staticpos-justify-self-img-002.html
- abspos/grid-abspos-staticpos-justify-self-img-last-baseline-001.html
- abspos/grid-abspos-staticpos-justify-self-img-last-baseline-002.html
- abspos/grid-abspos-staticpos-justify-self-last-baseline-001.html
- abspos/grid-abspos-staticpos-justify-self-last-baseline-002.html
- abspos/grid-abspos-staticpos-justify-self-rtl-last-baseline-001.html
- abspos/grid-abspos-staticpos-justify-self-rtl-last-baseline-002.html
- abspos/grid-abspos-staticpos-justify-self-rtl-last-baseline-003.html
- abspos/grid-abspos-staticpos-justify-self-rtl-last-baseline-004.html
- abspos/grid-fixedpos-static-position-not-containing-block.html
- abspos/grid-positioned-items-background-001.html
- abspos/orthogonal-positioned-grid-items-001.html
- abspos/orthogonal-positioned-grid-items-002.html
- abspos/orthogonal-positioned-grid-items-003.html
- abspos/orthogonal-positioned-grid-items-004.html
- abspos/orthogonal-positioned-grid-items-006.html
- abspos/orthogonal-positioned-grid-items-007.html
- abspos/orthogonal-positioned-grid-items-008.html
- abspos/orthogonal-positioned-grid-items-009.html
- abspos/orthogonal-positioned-grid-items-011.html
- abspos/orthogonal-positioned-grid-items-013.html
- abspos/positioned-grid-descendant-containing-block-001.html
- abspos/positioned-grid-descendant-containing-block-002.html
- abspos/positioned-grid-descendant-containing-block-003.html
- abspos/positioned-grid-descendant-containing-block-004.html
- abspos/positioned-grid-descendant-containing-block-005.html
- abspos/positioned-grid-descendant-containing-block-006.html
- abspos/positioned-grid-descendant-containing-block-007.html
- abspos/positioned-grid-descendant-containing-block-008.html
- abspos/positioned-grid-descendant-containing-block-011.html
- abspos/positioned-grid-descendant-containing-block-012.html
- abspos/positioned-grid-descendant-containing-block-016.html
- abspos/positioned-grid-items-001.html
- abspos/positioned-grid-items-002.html
- abspos/positioned-grid-items-003.html
- abspos/positioned-grid-items-004.html
- abspos/positioned-grid-items-005.html
- abspos/positioned-grid-items-006.html
- abspos/positioned-grid-items-007.html
- abspos/positioned-grid-items-008.html
- abspos/positioned-grid-items-009.html
- abspos/positioned-grid-items-010.html
- abspos/positioned-grid-items-011.html
- abspos/positioned-grid-items-012.html
- abspos/positioned-grid-items-013.html
- abspos/positioned-grid-items-014.html
- abspos/positioned-grid-items-015.html
- abspos/positioned-grid-items-016.html
- abspos/positioned-grid-items-017.html
- abspos/positioned-grid-items-018.html
- abspos/positioned-grid-items-019.html
- abspos/positioned-grid-items-020.html
- abspos/positioned-grid-items-021.html
- abspos/positioned-grid-items-022.html
- abspos/positioned-grid-items-024.html
- abspos/positioned-grid-items-025.html
- abspos/positioned-grid-items-026.html
- abspos/positioned-grid-items-negative-indices-001.html
- abspos/positioned-grid-items-negative-indices-002.html
- abspos/positioned-grid-items-negative-indices-003.html
- alignment/grid-align-content-end-001.html
- alignment/grid-align-content-space-between-001.html
- alignment/grid-baseline-002.html
- alignment/grid-baseline-003.html
- alignment/grid-block-axis-alignment-auto-margins-008.html
- alignment/grid-block-axis-alignment-auto-margins-explicit-row.html
- alignment/grid-calc-margins-explicit-row.html
- alignment/grid-content-distribution-001.html
- alignment/grid-content-distribution-002.html
- alignment/grid-content-distribution-003.html
- alignment/grid-content-distribution-004.html
- alignment/grid-content-distribution-005.html
- alignment/grid-content-distribution-006.html
- alignment/grid-content-distribution-007.html
- alignment/grid-content-distribution-008.html
- alignment/grid-content-distribution-009.html
- alignment/grid-content-distribution-010.html
- alignment/grid-content-distribution-011.html
- alignment/grid-content-distribution-012.html
- alignment/grid-content-distribution-013.html
- alignment/grid-content-distribution-014.html
- alignment/grid-content-distribution-015.html
- alignment/grid-content-distribution-016.html
- alignment/grid-content-distribution-017.html
- alignment/grid-content-distribution-018.html
- alignment/grid-content-distribution-019.html
- alignment/grid-content-distribution-020.html
- alignment/grid-content-distribution-021.html
- alignment/grid-content-distribution-022.html
- alignment/grid-content-distribution-023.html
- alignment/grid-content-distribution-024.html
- alignment/grid-content-distribution-025.html
- alignment/grid-content-distribution-026.html
- alignment/grid-content-distribution-027.html
- alignment/grid-content-distribution-028.html
- alignment/grid-content-distribution-029.html
- alignment/grid-content-distribution-with-collapsed-tracks-001.html
- alignment/grid-content-distribution-with-collapsed-tracks-002.html
- alignment/grid-content-distribution-with-collapsed-tracks-003.html
- alignment/grid-content-distribution-with-collapsed-tracks-004.html
- alignment/grid-content-distribution-with-collapsed-tracks-005.html
- alignment/grid-content-distribution-with-collapsed-tracks-006.html
- alignment/grid-content-distribution-with-collapsed-tracks-007.html
- alignment/grid-content-distribution-with-collapsed-tracks-008.html
- alignment/grid-content-distribution-with-collapsed-tracks-009.html
- alignment/grid-content-distribution-with-collapsed-tracks-010.html
- alignment/grid-content-distribution-with-collapsed-tracks-011.html
- alignment/grid-content-distribution-with-collapsed-tracks-012.html
- alignment/grid-content-distribution-with-collapsed-tracks-013.html
- alignment/grid-content-distribution-with-collapsed-tracks-014.html
- alignment/grid-content-distribution-with-collapsed-tracks-015.html
- alignment/grid-content-distribution-with-collapsed-tracks-016.html
- alignment/grid-content-distribution-with-collapsed-tracks-017.html
- alignment/grid-content-distribution-with-collapsed-tracks-018.html
- alignment/grid-content-distribution-with-collapsed-tracks-019.html
- alignment/grid-content-distribution-with-collapsed-tracks-020.html
- alignment/grid-content-distribution-with-collapsed-tracks-021.html
- alignment/grid-content-distribution-with-collapsed-tracks-022.html
- alignment/grid-content-distribution-with-collapsed-tracks-023.html
- alignment/grid-content-distribution-with-collapsed-tracks-024.html
- alignment/grid-gutters-001.html
- alignment/grid-gutters-002.html
- alignment/grid-gutters-003.html
- alignment/grid-gutters-004.html
- alignment/grid-gutters-005.html
- alignment/grid-gutters-006.html
- alignment/grid-gutters-007.html
- alignment/grid-gutters-008.html
- alignment/grid-gutters-011.html
- alignment/grid-gutters-012.html
- alignment/grid-gutters-013.html
- alignment/grid-gutters-explicit-placement.html
- alignment/grid-inline-axis-alignment-auto-margins-008.html
- alignment/grid-item-auto-margins-001.html
- alignment/grid-item-auto-margins-002.html
- alignment/grid-justify-content-end-001.html
- alignment/grid-justify-content-space-between-001.html
- alignment/grid-self-alignment-baseline-gfc-001.html
- alignment/grid-self-alignment-baseline-with-grid-001.html
- alignment/grid-self-alignment-baseline-with-grid-002.html
- alignment/grid-self-alignment-baseline-with-grid-003.html
- alignment/grid-self-alignment-baseline-with-grid-004.html
- anonymous-grid-items-001.html
- child-border-box-and-max-content-001.html
- child-border-box-and-max-content-002.html
- empty-grid-within-flexbox.html
- grid-calc-margin.html
- grid-child-percent-basis-resize-1.html
- grid-container-as-flex-item-001.html
- grid-container-as-flex-item-002.html
- grid-container-as-flex-item-003.html
- grid-definition/flex-item-grid-container-auto-repeat-001.html
- grid-definition/flex-item-grid-container-auto-repeat-002.html
- grid-definition/flex-item-grid-container-percentage-rows-001.html
- grid-definition/grid-auto-fit-with-calc.html
- grid-definition/grid-auto-repeat-minmax.html
- grid-definition/grid-auto-repeat-multiple-values-001.html
- grid-definition/grid-auto-repeat-multiple-values-002.html
- grid-definition/grid-auto-repeat-multiple-values-003.html
- grid-definition/grid-auto-repeat-multiple-values-004.html
- grid-definition/grid-auto-repeat-multiple-values-005.html
- grid-definition/grid-item-grid-container-percentage-rows-001.html
- grid-definition/grid-layout-auto-tracks.html
- grid-definition/grid-layout-basic.html
- grid-definition/grid-layout-repeat-notation.html
- grid-definition/grid-repeat-integer-001.html
- grid-definition/grid-repeat-integer-002.html
- grid-definition/grid-repeat-integer-003.html
- grid-definition/grid-repeat-integer-004.html
- grid-definition/grid-repeat-integer-005.html
- grid-definition/grid-support-named-grid-lines-002.html
- grid-definition/grid-support-named-grid-lines-003.html
- grid-flexible-track-fractional-flex-factor.html
- grid-flexible-track-free-space-distribution.html
- grid-fragmentation-between-rows-001-print.tentative.html
- grid-item-non-auto-height-stretch-001.html
- grid-item-non-auto-height-stretch-002.html
- grid-item-non-auto-height-stretch-003.html
- grid-item-non-auto-height-stretch-004.html
- grid-items/anonymous-grid-item-001.html
- grid-items/explicitly-sized-grid-item-as-table.html
- grid-items/grid-inline-items-001.html
- grid-items/grid-inline-items-002.html
- grid-items/grid-inline-items-003.html
- grid-items/grid-inline-items-inline-blocks-001.html
- grid-items/grid-inline-order-property-auto-placement-001.html
- grid-items/grid-inline-order-property-auto-placement-002.html
- grid-items/grid-inline-order-property-auto-placement-003.html
- grid-items/grid-inline-order-property-auto-placement-004.html
- grid-items/grid-inline-order-property-auto-placement-005.html
- grid-items/grid-inline-order-property-painting-001.html
- grid-items/grid-inline-order-property-painting-002.html
- grid-items/grid-inline-order-property-painting-003.html
- grid-items/grid-inline-order-property-painting-004.html
- grid-items/grid-inline-order-property-painting-005.html
- grid-items/grid-inline-z-axis-ordering-001.html
- grid-items/grid-inline-z-axis-ordering-002.html
- grid-items/grid-inline-z-axis-ordering-003.html
- grid-items/grid-inline-z-axis-ordering-004.html
- grid-items/grid-inline-z-axis-ordering-005.html
- grid-items/grid-inline-z-axis-ordering-overlapped-items-001.html
- grid-items/grid-inline-z-axis-ordering-overlapped-items-002.html
- grid-items/grid-inline-z-axis-ordering-overlapped-items-003.html
- grid-items/grid-inline-z-axis-ordering-overlapped-items-004.html
- grid-items/grid-inline-z-axis-ordering-overlapped-items-005.html
- grid-items/grid-inline-z-axis-ordering-overlapped-items-006.html
- grid-items/grid-intrinsic-maximums.html
- grid-items/grid-item-block-axis-content-contribution-001.html
- grid-items/grid-item-block-axis-content-contribution-002.html
- grid-items/grid-item-block-axis-content-contribution-003.html
- grid-items/grid-item-border-padding-001.html
- grid-items/grid-item-border-padding-002.html
- grid-items/grid-item-border-padding-003.html
- grid-items/grid-item-border-padding-004.html
- grid-items/grid-item-border-padding-005.html
- grid-items/grid-item-containing-block-001.html
- grid-items/grid-item-containing-block-002.html
- grid-items/grid-item-containing-block-003.html
- grid-items/grid-item-containing-block-004.html
- grid-items/grid-item-margin-box-overflowing-grid-area-paint-001.html
- grid-items/grid-item-min-height-overrides-fixed-height-001.html
- grid-items/grid-item-min-size-auto-overflow-001.html
- grid-items/grid-item-min-size-auto-overflow-002.html
- grid-items/grid-item-min-width-overrides-fixed-width-001.html
- grid-items/grid-item-overflow-auto-max-height-percentage.html
- grid-items/grid-item-overflow-offscreen-painting-001.html
- grid-items/grid-item-percentage-size-in-positioned-grid-001.html
- grid-items/grid-items-001.html
- grid-items/grid-items-002.html
- grid-items/grid-items-003.html
- grid-items/grid-items-calc-paddings-001.html
- grid-items/grid-items-contribution-negative-margins.html
- grid-items/grid-items-em-paddings-001.html
- grid-items/grid-items-fixed-paddings-001.html
- grid-items/grid-items-inline-blocks-001.html
- grid-items/grid-items-painted-once-001.html
- grid-items/grid-items-painting-order-dom-001.html
- grid-items/grid-items-percentage-margins-and-fixed-paddings-001.html
- grid-items/grid-items-percentage-paddings-016.html
- grid-items/grid-items-relative-positioned-containing-block-001.html
- grid-items/grid-items-relative-positioned-containing-block-002.html
- grid-items/grid-items-relative-positioned-containing-block-003.html
- grid-items/grid-layout-grid-in-grid.html
- grid-items/grid-minimum-size-grid-items-001.html
- grid-items/grid-minimum-size-grid-items-002.html
- grid-items/grid-minimum-size-grid-items-003.html
- grid-items/grid-minimum-size-grid-items-004.html
- grid-items/grid-minimum-size-grid-items-005.html
- grid-items/grid-minimum-size-grid-items-006.html
- grid-items/grid-minimum-size-grid-items-007.html
- grid-items/grid-minimum-size-grid-items-008.html
- grid-items/grid-minimum-size-grid-items-009.html
- grid-items/grid-minimum-size-grid-items-010.html
- grid-items/grid-minimum-size-grid-items-011.html
- grid-items/grid-minimum-size-grid-items-012.html
- grid-items/grid-minimum-size-grid-items-013.html
- grid-items/grid-minimum-size-grid-items-014.html
- grid-items/grid-minimum-size-grid-items-015.html
- grid-items/grid-minimum-size-grid-items-016.html
- grid-items/grid-minimum-size-grid-items-017.html
- grid-items/grid-minimum-size-grid-items-018.html
- grid-items/grid-minimum-size-grid-items-019.html
- grid-items/grid-minimum-size-grid-items-020.html
- grid-items/grid-order-property-auto-placement-001.html
- grid-items/grid-order-property-auto-placement-002.html
- grid-items/grid-order-property-auto-placement-003.html
- grid-items/grid-order-property-auto-placement-004.html
- grid-items/grid-order-property-auto-placement-005.html
- grid-items/grid-order-property-painting-001.html
- grid-items/grid-order-property-painting-002.html
- grid-items/grid-order-property-painting-003.html
- grid-items/grid-order-property-painting-004.html
- grid-items/grid-order-property-painting-005.html
- grid-items/grid-z-axis-ordering-001.html
- grid-items/grid-z-axis-ordering-002.html
- grid-items/grid-z-axis-ordering-003.html
- grid-items/grid-z-axis-ordering-004.html
- grid-items/grid-z-axis-ordering-005.html
- grid-items/grid-z-axis-ordering-overlapped-items-001.html
- grid-items/grid-z-axis-ordering-overlapped-items-002.html
- grid-items/grid-z-axis-ordering-overlapped-items-003.html
- grid-items/grid-z-axis-ordering-overlapped-items-004.html
- grid-items/grid-z-axis-ordering-overlapped-items-005.html
- grid-items/grid-z-axis-ordering-overlapped-items-006.html
- grid-items/item-with-table-with-infinite-max-intrinsic-width.html
- grid-items/overflow-hidden-min-content-contribution-in-abs-pos-grid-in-flexbox.html
- grid-items/percentage-size-subitems-002.html
- grid-items/replaced-element-001.html
- grid-items/replaced-element-002.html
- grid-items/replaced-element-007.html
- grid-items/replaced-element-010.html
- grid-items/table-with-infinite-max-intrinsic-width.html
- grid-items/whitespace-in-grid-item-001.html
- grid-model/column-property-should-not-apply-on-grid-container-001.html
- grid-model/grid-areas-overflowing-grid-container-004.html
- grid-model/grid-areas-overflowing-grid-container-005.html
- grid-model/grid-areas-overflowing-grid-container-006.html
- grid-model/grid-areas-overflowing-grid-container-007.html
- grid-model/grid-areas-overflowing-grid-container-008.html
- grid-model/grid-container-ignores-first-letter-002.html
- grid-model/grid-container-scrollbar-001.html
- grid-model/grid-container-scrollbars-sizing-001.html
- grid-model/grid-container-scrollbars-sizing-002.html
- grid-model/grid-display-grid-001.html
- grid-model/grid-display-inline-grid-001.html
- grid-model/grid-empty-floating-positioned-and-inline-001.html
- grid-model/grid-first-letter-001.html
- grid-model/grid-first-letter-002.html
- grid-model/grid-first-line-001.html
- grid-model/grid-first-line-002.html
- grid-model/grid-float-001.html
- grid-model/grid-floats-no-intrude-001.html
- grid-model/grid-inline-first-letter-001.html
- grid-model/grid-inline-first-letter-002.html
- grid-model/grid-inline-first-line-001.html
- grid-model/grid-inline-first-line-002.html
- grid-model/grid-inline-float-001.html
- grid-model/grid-inline-margins-no-collapse-001.html
- grid-model/grid-inline-multicol-001.html
- grid-model/grid-inline-vertical-align-001.html
- grid-model/grid-margins-no-collapse-001.html
- grid-model/grid-margins-no-collapse-002.html
- grid-model/grid-multicol-001.html
- grid-model/grid-painting-rtl-001.html
- grid-model/grid-vertical-align-001.html
- grid-overflowing-item-below-container-is-painted.html
- grid-overflowing-item-content-is-painted.html
- grid-overflowing-item-in-both-axes-is-painted.html
- grid-overflowing-item-in-nonstart-cell-is-painted.html
- grid-overflowing-item-right-of-container-is-painted.html
- grid-overflowing-multiple-items-are-painted.html
- grid-within-flexbox-definite-change.html
- grid-within-flexbox-indefinite.html
- implicit-grids/grid-support-grid-auto-columns-rows-001.html
- implicit-grids/grid-support-grid-auto-columns-rows-002.html
- layout-algorithm/auto-margins-ignored-during-track-sizing-001.html
- layout-algorithm/flex-and-intrinsic-sizes-002.html
- layout-algorithm/grid-as-flex-item-should-not-shrink-to-fit-001.html
- layout-algorithm/grid-as-flex-item-should-not-shrink-to-fit-002.html
- layout-algorithm/grid-as-flex-item-should-not-shrink-to-fit-005.html
- layout-algorithm/grid-as-flex-item-should-not-shrink-to-fit-006.html
- layout-algorithm/grid-as-flex-item-should-not-shrink-to-fit-007.html
- layout-algorithm/grid-as-flex-item-should-not-shrink-to-fit-008.html
- layout-algorithm/grid-auto-track-item-with-border-001.html
- layout-algorithm/grid-auto-tracks-items-with-borders-001.html
- layout-algorithm/grid-find-fr-size-restart-algorithm.html
- layout-algorithm/grid-flex-and-auto-columns-fill-available-width-001.html
- layout-algorithm/grid-intrinsic-size-nested-baseline-grid-001.html
- layout-algorithm/grid-intrinsic-track-sizes-min-size-001.html
- layout-algorithm/grid-layout-free-space-unit.html
- layout-algorithm/grid-max-content-tracks-items-with-paddings-001.html
- layout-algorithm/grid-percent-cols-filled-shrinkwrap-001.html
- layout-algorithm/grid-percent-cols-spanned-shrinkwrap-001.html
- layout-algorithm/grid-spanning-item-with-paddings-auto-columns-001.html
- layout-algorithm/grid-spanning-item-with-paddings-auto-rows-001.html
- layout-algorithm/grid-stretch-respects-min-size-001.html
- layout-algorithm/grid-track-ignores-max-size-001.html
- min-size-auto-overflow-clip.html
- nested-grid-item-block-size-001.html
- placement/grid-layout-grid-span.html
- placement/grid-layout-lines-shorthands.html
- placement/grid-layout-lines.html
- placement/grid-layout-placement-shorthands.html
- placement/grid-placement-items-spanning-multiple-rows-001.html
- placement/grid-placement-items-spanning-multiple-rows-002.html
- placement/grid-placement-using-named-grid-lines-001.html
- placement/grid-placement-using-named-grid-lines-002.html
- placement/grid-placement-using-named-grid-lines-003.html
- placement/grid-placement-using-named-grid-lines-004.html
- placement/grid-placement-using-named-grid-lines-005.html
- placement/grid-placement-using-named-grid-lines-006.html
- placement/grid-placement-using-named-grid-lines-008.html
- placement/grid-placement-using-named-grid-lines-009.html
- placement/grid-template-areas-media-query-001.html
- placement/grid-template-areas-must-keep-named-columns-order-001.html
- relative-grandchild.html
- replaced-element-percentage-height-in-grid-nested-in-flex-001.html
- replaced-element-percentage-height-in-grid-nested-in-flex-002.html
- stretch-grid-item-button-overflow.html
- subgrid/abs-pos-001.html
- subgrid/auto-track-sizing-001.html
- subgrid/auto-track-sizing-002.html
- subgrid/auto-track-sizing-003.html
- subgrid/independent-tracks-from-parent-grid.html
- subgrid/line-names-002.html
- subgrid/line-names-003.html
- subgrid/line-names-007.html
- subgrid/line-names-013.html
- subgrid/line-names-014.html
- subgrid/line-names-015.html
- subgrid/line-names-parent-auto-repeat-001.html
- subgrid/orthogonal-writing-mode-005.html
- subgrid/orthogonal-writing-mode-006.html
- subgrid/percentage-track-sizing.html
- subgrid/repeat-auto-fill-007.html
- subgrid/standalone-axis-size-001.html
- subgrid/standalone-axis-size-002.html
- subgrid/standalone-axis-size-003.html
- subgrid/standalone-axis-size-004.html
- subgrid/standalone-axis-size-006.html
- subgrid/standalone-axis-size-008.html
- subgrid/standalone-axis-size-009.html
- subgrid/subgrid-baseline-012.html
- subgrid/subgrid-no-items-on-edges-001.html
- subgrid/subgrid-scroller-auto-height-padding.html
- subgrid/writing-directions-002.html
