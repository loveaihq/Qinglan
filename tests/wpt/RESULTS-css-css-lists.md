# WPT `css/css-lists`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/css-lists`，用时 0.9 秒。

- 通过 **80 / 143**（55.9%）
- bug 类失败：**15**
- 跳过：needs JS 28，no reftest 48

失败分类：bug (open) 15，limit (::marker style) 8，limit (css-lists-3 reversed()) 32，limit (list-style-image) 1，step 2b 2，step 2c 2，step 2d 3

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| content-property/marker-text-matches-circle.html | limit (::marker style) | ::marker supports only `content` (and list-style-type); the test needs the marker box styled (color, font, text-fill) or compared glyph for glyph with the built-in marker (1280 pixels differ (largest difference 255); first at (16,0): #000000, the reference has #ffffff) |
| content-property/marker-text-matches-disc.html | limit (::marker style) | ::marker supports only `content` (and list-style-type); the test needs the marker box styled (color, font, text-fill) or compared glyph for glyph with the built-in marker (1280 pixels differ (largest difference 255); first at (16,0): #000000, the reference has #ffffff) |
| content-property/marker-text-matches-square.html | limit (::marker style) | ::marker supports only `content` (and list-style-type); the test needs the marker box styled (color, font, text-fill) or compared glyph for glyph with the built-in marker (1280 pixels differ (largest difference 255); first at (16,0): #000000, the reference has #ffffff) |
| counter-002.html | bug (open) | css-lists-3 4.4.1 counter inheritance (a following sibling inherits the previous sibling counters, then value-source copy) is approximated by a scope stack with the sibling-hiding rule; this page differs from the first nested or sibling-reset counter on (23808 pixels differ (largest difference 255); first at (216,64): #000000, the reference has #ffffff) |
| counter-003.html | bug (open) | css-lists-3 4.4.1 counter inheritance (a following sibling inherits the previous sibling counters, then value-source copy) is approximated by a scope stack with the sibling-hiding rule; this page differs from the first nested or sibling-reset counter on (23808 pixels differ (largest difference 255); first at (216,64): #000000, the reference has #ffffff) |
| counter-004.html | bug (open) | css-lists-3 4.4.1 counter inheritance (a following sibling inherits the previous sibling counters, then value-source copy) is approximated by a scope stack with the sibling-hiding rule; this page differs from the first nested or sibling-reset counter on (28160 pixels differ (largest difference 255); first at (312,48): #000000, the reference has #ffffff) |
| counter-list-item.html | bug (open) | css-lists-3 4.4.1 counter inheritance (a following sibling inherits the previous sibling counters, then value-source copy) is approximated by a scope stack with the sibling-hiding rule; this page differs from the first nested or sibling-reset counter on (3584 pixels differ (largest difference 255); first at (184,280): #ffffff, the reference has #000000) |
| counter-reset-increment-set-display-contents.html | bug (open) | css-lists-3 4.4.1 counter inheritance (a following sibling inherits the previous sibling counters, then value-source copy) is approximated by a scope stack with the sibling-hiding rule; this page differs from the first nested or sibling-reset counter on (512 pixels differ (largest difference 255); first at (24,48): #000000, the reference has #ffffff) |
| counter-reset-reversed-display-none.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (512 pixels differ (largest difference 255); first at (24,8): #000000, the reference has #ffffff) |
| counter-reset-reversed-multiple.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (1024 pixels differ (largest difference 255); first at (56,8): #000000, the reference has #ffffff) |
| counter-reset-reversed-nested.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (2304 pixels differ (largest difference 255); first at (80,16): #000000, the reference has #ffffff) |
| counter-reset-reversed-pseudo-001.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (1024 pixels differ (largest difference 255); first at (72,8): #000000, the reference has #ffffff) |
| counter-reset-reversed-pseudo-002.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (256 pixels differ (largest difference 255); first at (24,8): #000000, the reference has #ffffff) |
| counter-reset-reversed-pseudo-003.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (512 pixels differ (largest difference 255); first at (72,8): #000000, the reference has #ffffff) |
| counter-reset-reversed-siblings-001a.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (512 pixels differ (largest difference 255); first at (24,8): #000000, the reference has #ffffff) |
| counter-reset-reversed-siblings-001b.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (512 pixels differ (largest difference 255); first at (24,8): #000000, the reference has #ffffff) |
| counter-reset-reversed-siblings-002.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (512 pixels differ (largest difference 255); first at (24,8): #000000, the reference has #ffffff) |
| counter-reset-reversed-with-regular.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (256 pixels differ (largest difference 255); first at (56,48): #000000, the reference has #ffffff) |
| counters-001.html | bug (open) | css-lists-3 4.4.1 counter inheritance (a following sibling inherits the previous sibling counters, then value-source copy) is approximated by a scope stack with the sibling-hiding rule; this page differs from the first nested or sibling-reset counter on (10496 pixels differ (largest difference 255); first at (136,48): #000000, the reference has #ffffff) |
| counters-002.html | bug (open) | css-lists-3 4.4.1 counter inheritance (a following sibling inherits the previous sibling counters, then value-source copy) is approximated by a scope stack with the sibling-hiding rule; this page differs from the first nested or sibling-reset counter on (38656 pixels differ (largest difference 255); first at (104,48): #000000, the reference has #ffffff) |
| counters-003.html | bug (open) | css-lists-3 4.4.1 counter inheritance (a following sibling inherits the previous sibling counters, then value-source copy) is approximated by a scope stack with the sibling-hiding rule; this page differs from the first nested or sibling-reset counter on (38656 pixels differ (largest difference 255); first at (104,48): #000000, the reference has #ffffff) |
| counters-004.html | bug (open) | css-lists-3 4.4.1 counter inheritance (a following sibling inherits the previous sibling counters, then value-source copy) is approximated by a scope stack with the sibling-hiding rule; this page differs from the first nested or sibling-reset counter on (55808 pixels differ (largest difference 255); first at (88,48): #000000, the reference has #ffffff) |
| counters-scope-002.html | bug (open) | css-lists-3 4.4.1 counter inheritance (a following sibling inherits the previous sibling counters, then value-source copy) is approximated by a scope stack with the sibling-hiding rule; this page differs from the first nested or sibling-reset counter on (2816 pixels differ (largest difference 255); first at (728,48): #000000, the reference has #ffffff) |
| counters-scope-003.html | bug (open) | css-lists-3 4.4.1 counter inheritance (a following sibling inherits the previous sibling counters, then value-source copy) is approximated by a scope stack with the sibling-hiding rule; this page differs from the first nested or sibling-reset counter on (1024 pixels differ (largest difference 255); first at (264,48): #000000, the reference has #ffffff) |
| counters-scope-004.html | bug (open) | css-lists-3 4.4.1 counter inheritance (a following sibling inherits the previous sibling counters, then value-source copy) is approximated by a scope stack with the sibling-hiding rule; this page differs from the first nested or sibling-reset counter on (1024 pixels differ (largest difference 255); first at (120,48): #000000, the reference has #ffffff) |
| foo-counter-reversed-006a.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (2740 pixels differ (largest difference 255); first at (8,20): #000000, the reference has #ffffff) |
| foo-counter-reversed-006b.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (2740 pixels differ (largest difference 255); first at (8,20): #000000, the reference has #ffffff) |
| foo-counter-reversed-006c.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (2740 pixels differ (largest difference 255); first at (8,20): #000000, the reference has #ffffff) |
| foo-counter-reversed-006d.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (2740 pixels differ (largest difference 255); first at (8,20): #000000, the reference has #ffffff) |
| foo-counter-reversed-006e.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (2740 pixels differ (largest difference 255); first at (8,20): #000000, the reference has #ffffff) |
| foo-counter-reversed-007a.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (9920 pixels differ (largest difference 255); first at (8,20): #000000, the reference has #ffffff) |
| foo-counter-reversed-007b.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (9920 pixels differ (largest difference 255); first at (8,20): #000000, the reference has #ffffff) |
| foo-counter-reversed-008a.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (1300 pixels differ (largest difference 255); first at (58,10): #ffffff, the reference has #000000) |
| foo-counter-reversed-008b.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (1300 pixels differ (largest difference 255); first at (58,10): #ffffff, the reference has #000000) |
| foo-counter-reversed-009a.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (5100 pixels differ (largest difference 255); first at (8,20): #000000, the reference has #ffffff) |
| foo-counter-reversed-009b.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (5100 pixels differ (largest difference 255); first at (8,20): #000000, the reference has #ffffff) |
| inline-block-list-marker.html | limit (::marker style) | ::marker supports only `content` (and list-style-type); the test needs the marker box styled (color, font, text-fill) or compared glyph for glyph with the built-in marker (38796 pixels differ (largest difference 255); first at (56,30): #000000, the reference has #ffffff) |
| inline-block-list.html | bug (open) | marker of an inline-level list item (display:inline / inline-block list-item): css-lists-3 3.1 gives them no outside marker; the engine draws one at the wrong place; not related to floats (40140 pixels differ (largest difference 255); first at (56,30): #000000, the reference has #ffffff) |
| inline-list-marker.html | limit (::marker style) | ::marker supports only `content` (and list-style-type); the test needs the marker box styled (color, font, text-fill) or compared glyph for glyph with the built-in marker (51113 pixels differ (largest difference 255); first at (57,0): #000000, the reference has #ffffff) |
| inline-list-with-table-child.html | bug (open) | used to be classed "2a-2 tables" (the page has a table); it still fails now that tables exist (first difference: (25,25): #ffffff, the reference has #000000); not analyzed one by one (21176 pixels differ (largest difference 255); first at (25,25): #ffffff, the reference has #000000) |
| inline-list.html | bug (open) | marker of an inline-level list item (display:inline / inline-block list-item): css-lists-3 3.1 gives them no outside marker; the engine draws one at the wrong place; not related to floats (51933 pixels differ (largest difference 255); first at (55,0): #000000, the reference has #ffffff) |
| li-value-reversed-006a.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (2360 pixels differ (largest difference 255); first at (8,40): #000000, the reference has #ffffff) |
| li-value-reversed-006b.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (2740 pixels differ (largest difference 255); first at (8,20): #000000, the reference has #ffffff) |
| li-value-reversed-006c.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (2360 pixels differ (largest difference 255); first at (8,40): #000000, the reference has #ffffff) |
| li-value-reversed-006d.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (2740 pixels differ (largest difference 255); first at (8,20): #000000, the reference has #ffffff) |
| li-value-reversed-006e.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (2740 pixels differ (largest difference 255); first at (8,20): #000000, the reference has #ffffff) |
| li-value-reversed-007a.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (6260 pixels differ (largest difference 255); first at (118,40): #ffffff, the reference has #000000) |
| li-value-reversed-007b.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (6260 pixels differ (largest difference 255); first at (118,40): #ffffff, the reference has #000000) |
| li-value-reversed-008a.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (700 pixels differ (largest difference 255); first at (58,10): #ffffff, the reference has #000000) |
| li-value-reversed-008b.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (700 pixels differ (largest difference 255); first at (58,10): #ffffff, the reference has #000000) |
| li-value-reversed-009a.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (1400 pixels differ (largest difference 255); first at (118,230): #ffffff, the reference has #000000) |
| li-value-reversed-009b.html | limit (css-lists-3 reversed()) | uses counter-reset: reversed(name) or an <ol reversed> with reversed() counters (css-lists-3 4.4.1/4.5): not in the 2a-1 task (counter-reset/increment/set of CSS2 12.4 only); the plain counter-reset name N form works (1600 pixels differ (largest difference 255); first at (68,170): #000000, the reference has #ffffff) |
| list-style-type-decimal-vertical-lr.html | step 2c | uses painting effects (17600 pixels differ (largest difference 255); first at (8,40): #000000, the reference has #ffffff) |
| list-style-type-decimal-vertical-rl.html | step 2c | uses painting effects (17600 pixels differ (largest difference 255); first at (8,40): #000000, the reference has #ffffff) |
| list-style-type-string-002.html | step 2d | uses text features (21504 pixels differ (largest difference 255); first at (0,32): #000000, the reference has #ffffff) |
| list-style-type-string-003.html | step 2d | uses text features (21504 pixels differ (largest difference 255); first at (0,32): #000000, the reference has #ffffff) |
| list-style-type-string-005a.html | step 2b | uses flex or grid (19456 pixels differ (largest difference 255); first at (200,12): #000000, the reference has #ffffff) |
| list-style-type-string-005b.html | step 2b | uses flex or grid (48640 pixels differ (largest difference 255); first at (200,12): #000000, the reference has #ffffff) |
| list-style-type-string-007.html | step 2d | uses text features (5660 pixels differ (largest difference 255); first at (188,18): #ffffff, the reference has #000000) |
| list-type-none-style-image.html | limit (list-style-image) | list-style-image is not supported (an image marker is a later step) (256 pixels differ (largest difference 255); first at (16,16): #ffffff, the reference has #000000) |
| marker-counter.html | limit (::marker style) | ::marker supports only `content` (and list-style-type); the test needs the marker box styled (color, font, text-fill) or compared glyph for glyph with the built-in marker (2432 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #000000) |
| marker-webkit-text-fill-color.html | limit (::marker style) | ::marker supports only `content` (and list-style-type); the test needs the marker box styled (color, font, text-fill) or compared glyph for glyph with the built-in marker (28835 pixels differ (largest difference 255); first at (8,13): #ff0000, the reference has #008000) |
| nested-marker-styling.html | limit (::marker style) | ::marker supports only `content` (and list-style-type); the test needs the marker box styled (color, font, text-fill) or compared glyph for glyph with the built-in marker (5888 pixels differ (largest difference 255); first at (24,8): #ffffff, the reference has #000000) |

## 通过

- content-property/marker-text-matches-armenian.html
- content-property/marker-text-matches-decimal-leading-zero.html
- content-property/marker-text-matches-decimal.html
- content-property/marker-text-matches-georgian.html
- content-property/marker-text-matches-lower-greek.html
- content-property/marker-text-matches-lower-latin.html
- content-property/marker-text-matches-lower-roman.html
- content-property/marker-text-matches-upper-latin.html
- content-property/marker-text-matches-upper-roman.html
- counter-001.html
- counter-increment-inside-display-contents.html
- counter-invalid.htm
- counter-list-item-2.html
- counter-list-item-3.html
- counter-order-display-contents.html
- counter-reset-increment-overflow-underflow.html
- counter-reset-increment-set-display-none.html
- counter-reset-inside-display-contents.html
- counter-reset-reversed-list-item-start.html
- counter-reset-reversed-list-item.html
- counter-reset-reversed-not-list-item-start.html
- counter-reset-reversed-not-list-item.html
- counter-reset-reversed-siblings-003.html
- counter-set-001.html
- counters-005.html
- counters-006.html
- counters-scope-001.html
- implicit-and-explicit-list-item-counters.html
- li-list-item-counter-001.html
- li-list-item-counter-002.html
- li-list-item-counter-003.html
- li-list-item-counter-004.html
- li-list-item-counter-005.html
- li-value-counter-reset-001.html
- li-value-reversed-001.html
- li-value-reversed-002.html
- li-value-reversed-003.html
- li-value-reversed-004.html
- li-value-reversed-005.html
- li-value-reversed-010.html
- li-value-reversed-011.html
- li-value-reversed-012.html
- li-value-reversed-013.html
- li-value-reversed-014.html
- li-value-reversed-015.html
- li-value-reversed-016.html
- li-value-reversed-017.html
- li-value-reversed-018.html
- li-value-reversed-019.html
- li-value-reversed-020.html
- li-value-reversed-021.html
- li-value-reversed-022.html
- li-value-reversed-023.html
- li-value-reversed-024.html
- li-value-reversed-025.html
- li-value-reversed-026.html
- li-value-reversed-027.html
- li-value-reversed-028.html
- li-value-reversed-029.html
- li-value-reversed-030.html
- list-and-block-in-inline.html
- list-and-flex-001.html
- list-and-grid-001.html
- list-item-definition.html
- list-marker-alignment.html
- list-marker-counter-style-fallback-rtl.html
- list-marker-rtl-string-width.html
- list-marker-symbol-bidi.html
- list-marker-with-lineheight-and-overflow-hidden-001.html
- list-style-image-gradients.html
- list-style-position-001.html
- list-style-type-decimal-line-height.html
- list-style-type-string-001a.html
- list-style-type-string-001b.html
- list-style-type-string-006.html
- marker-quotes.html
- nested-marker.html
- outside-marker-covered-by-relpos-block-link.html
- outside-marker-in-multicol-relpos-content.html
- outside-marker-with-relpos-block-content.html
