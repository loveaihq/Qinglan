# WPT `css/css-lists`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/css-lists`，用时 12.4 秒。

- 通过 **68 / 143**（47.6%）
- bug 类失败：**2**
- 跳过：needs JS 28，no reftest 48

失败分类：bug (open) 2，step 2a-1 (generated content, not done) 71，step 2a-2 1，step 2d 1

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| content-property/marker-text-matches-circle.html | step 2a-1 (generated content, not done) | uses generated content (1536 pixels differ (largest difference 255); first at (32,0): #ffffff, the reference has #000000) |
| content-property/marker-text-matches-disc.html | step 2a-1 (generated content, not done) | uses generated content (1536 pixels differ (largest difference 255); first at (32,0): #ffffff, the reference has #000000) |
| content-property/marker-text-matches-square.html | step 2a-1 (generated content, not done) | uses generated content (1536 pixels differ (largest difference 255); first at (32,0): #ffffff, the reference has #000000) |
| counter-001.html | step 2a-1 (generated content, not done) | uses generated content (7680 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #000000) |
| counter-002.html | step 2a-1 (generated content, not done) | uses generated content (15872 pixels differ (largest difference 255); first at (8,80): #ffffff, the reference has #000000) |
| counter-003.html | step 2a-1 (generated content, not done) | uses generated content (15872 pixels differ (largest difference 255); first at (8,80): #ffffff, the reference has #000000) |
| counter-004.html | step 2a-1 (generated content, not done) | uses generated content (11776 pixels differ (largest difference 255); first at (8,80): #ffffff, the reference has #000000) |
| counter-increment-inside-display-contents.html | step 2a-1 (generated content, not done) | uses generated content (256 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #000000) |
| counter-list-item.html | step 2a-1 (generated content, not done) | uses generated content (27648 pixels differ (largest difference 255); first at (80,136): #ffffff, the reference has #000000) |
| counter-reset-increment-overflow-underflow.html | step 2a-1 (generated content, not done) | uses generated content (22401 pixels differ (largest difference 255); first at (8,19): #ffffff, the reference has #000000) |
| counter-reset-increment-set-display-contents.html | step 2a-1 (generated content, not done) | uses generated content (256 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #000000) |
| counter-reset-increment-set-display-none.html | step 2a-1 (generated content, not done) | uses generated content (256 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #000000) |
| counter-reset-inside-display-contents.html | step 2a-1 (generated content, not done) | uses generated content (256 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #000000) |
| counter-reset-reversed-list-item-start.html | step 2a-1 (generated content, not done) | uses generated content (256 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #000000) |
| counter-reset-reversed-list-item.html | step 2a-1 (generated content, not done) | uses generated content (256 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #000000) |
| counter-reset-reversed-not-list-item-start.html | step 2a-1 (generated content, not done) | uses generated content (256 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #000000) |
| counter-reset-reversed-not-list-item.html | step 2a-1 (generated content, not done) | uses generated content (512 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #000000) |
| counter-set-001.html | step 2a-1 (generated content, not done) | uses generated content (3328 pixels differ (largest difference 255); first at (0,0): #ffffff, the reference has #000000) |
| counters-001.html | step 2a-1 (generated content, not done) | uses generated content (9216 pixels differ (largest difference 255); first at (8,80): #ffffff, the reference has #000000) |
| counters-002.html | step 2a-1 (generated content, not done) | uses generated content (19712 pixels differ (largest difference 255); first at (8,80): #ffffff, the reference has #000000) |
| counters-003.html | step 2a-1 (generated content, not done) | uses generated content (19712 pixels differ (largest difference 255); first at (8,80): #ffffff, the reference has #000000) |
| counters-004.html | step 2a-1 (generated content, not done) | uses generated content (33792 pixels differ (largest difference 255); first at (8,112): #ffffff, the reference has #000000) |
| counters-005.html | step 2a-1 (generated content, not done) | uses generated content (1792 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #000000) |
| counters-006.html | step 2a-1 (generated content, not done) | uses generated content (6144 pixels differ (largest difference 255); first at (48,0): #ffffff, the reference has #000000) |
| counters-scope-001.html | step 2a-1 (generated content, not done) | uses generated content (3072 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #000000) |
| counters-scope-002.html | step 2a-1 (generated content, not done) | uses generated content (13824 pixels differ (largest difference 255); first at (8,80): #ffffff, the reference has #000000) |
| counters-scope-003.html | step 2a-1 (generated content, not done) | uses generated content (5120 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #000000) |
| counters-scope-004.html | step 2a-1 (generated content, not done) | uses generated content (1280 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #000000) |
| foo-counter-reversed-006a.html | step 2a-1 (generated content, not done) | uses generated content (5700 pixels differ (largest difference 255); first at (48,10): #ffffff, the reference has #000000) |
| foo-counter-reversed-006b.html | step 2a-1 (generated content, not done) | uses generated content (5700 pixels differ (largest difference 255); first at (48,10): #ffffff, the reference has #000000) |
| foo-counter-reversed-006c.html | step 2a-1 (generated content, not done) | uses generated content (5700 pixels differ (largest difference 255); first at (48,10): #ffffff, the reference has #000000) |
| foo-counter-reversed-006d.html | step 2a-1 (generated content, not done) | uses generated content (5700 pixels differ (largest difference 255); first at (48,10): #ffffff, the reference has #000000) |
| foo-counter-reversed-006e.html | step 2a-1 (generated content, not done) | uses generated content (5700 pixels differ (largest difference 255); first at (48,10): #ffffff, the reference has #000000) |
| foo-counter-reversed-007a.html | step 2a-1 (generated content, not done) | uses generated content (380 pixels differ (largest difference 255); first at (0,10): #ffffff, the reference has #000000) |
| foo-counter-reversed-007b.html | step 2a-1 (generated content, not done) | uses generated content (380 pixels differ (largest difference 255); first at (0,10): #ffffff, the reference has #000000) |
| foo-counter-reversed-008a.html | step 2a-1 (generated content, not done) | uses generated content (5400 pixels differ (largest difference 255); first at (8,10): #ffffff, the reference has #000000) |
| foo-counter-reversed-008b.html | step 2a-1 (generated content, not done) | uses generated content (5400 pixels differ (largest difference 255); first at (8,10): #ffffff, the reference has #000000) |
| foo-counter-reversed-009a.html | step 2a-1 (generated content, not done) | uses generated content (200 pixels differ (largest difference 255); first at (8,10): #ffffff, the reference has #000000) |
| foo-counter-reversed-009b.html | step 2a-1 (generated content, not done) | uses generated content (200 pixels differ (largest difference 255); first at (8,10): #ffffff, the reference has #000000) |
| implicit-and-explicit-list-item-counters.html | step 2a-1 (generated content, not done) | uses generated content (4608 pixels differ (largest difference 255); first at (40,8): #000000, the reference has #ffffff) |
| inline-block-list-marker.html | step 2a-1 (generated content, not done) | uses generated content (40140 pixels differ (largest difference 255); first at (56,30): #000000, the reference has #ffffff) |
| inline-block-list.html | bug (open) | display:inline-block / inline list items: the marker of an inline-level list item (css-lists-3 3.1: only block-level list items get an outside marker) is drawn at the wrong place; not related to floats, not analysed further in 2a-1 (40140 pixels differ (largest difference 255); first at (56,30): #000000, the reference has #ffffff) |
| inline-list-marker.html | step 2a-1 (generated content, not done) | uses generated content (52265 pixels differ (largest difference 255); first at (57,0): #000000, the reference has #ffffff) |
| inline-list-with-table-child.html | step 2a-2 | uses tables (140693 pixels differ (largest difference 255); first at (25,25): #ffffff, the reference has #000000) |
| inline-list.html | bug (open) | display:inline list items: same as inline-block-list.html (51933 pixels differ (largest difference 255); first at (55,0): #000000, the reference has #ffffff) |
| li-list-item-counter-001.html | step 2a-1 (generated content, not done) | uses generated content (256 pixels differ (largest difference 255); first at (136,144): #ffffff, the reference has #000000) |
| li-list-item-counter-005.html | step 2a-1 (generated content, not done) | uses generated content (256 pixels differ (largest difference 255); first at (24,48): #ffffff, the reference has #000000) |
| li-value-counter-reset-001.html | step 2a-1 (generated content, not done) | uses generated content (1280 pixels differ (largest difference 255); first at (136,32): #ffffff, the reference has #000000) |
| li-value-reversed-006a.html | step 2a-1 (generated content, not done) | uses generated content (5700 pixels differ (largest difference 255); first at (48,10): #ffffff, the reference has #000000) |
| li-value-reversed-006b.html | step 2a-1 (generated content, not done) | uses generated content (5700 pixels differ (largest difference 255); first at (48,10): #ffffff, the reference has #000000) |
| li-value-reversed-006c.html | step 2a-1 (generated content, not done) | uses generated content (5700 pixels differ (largest difference 255); first at (48,10): #ffffff, the reference has #000000) |
| li-value-reversed-006d.html | step 2a-1 (generated content, not done) | uses generated content (5700 pixels differ (largest difference 255); first at (48,10): #ffffff, the reference has #000000) |
| li-value-reversed-006e.html | step 2a-1 (generated content, not done) | uses generated content (5700 pixels differ (largest difference 255); first at (48,10): #ffffff, the reference has #000000) |
| li-value-reversed-007a.html | step 2a-1 (generated content, not done) | uses generated content (380 pixels differ (largest difference 255); first at (0,10): #ffffff, the reference has #000000) |
| li-value-reversed-007b.html | step 2a-1 (generated content, not done) | uses generated content (380 pixels differ (largest difference 255); first at (0,10): #ffffff, the reference has #000000) |
| li-value-reversed-008a.html | step 2a-1 (generated content, not done) | uses generated content (5400 pixels differ (largest difference 255); first at (8,10): #ffffff, the reference has #000000) |
| li-value-reversed-008b.html | step 2a-1 (generated content, not done) | uses generated content (5400 pixels differ (largest difference 255); first at (8,10): #ffffff, the reference has #000000) |
| li-value-reversed-009a.html | step 2a-1 (generated content, not done) | uses generated content (200 pixels differ (largest difference 255); first at (8,10): #ffffff, the reference has #000000) |
| li-value-reversed-009b.html | step 2a-1 (generated content, not done) | uses generated content (200 pixels differ (largest difference 255); first at (8,10): #ffffff, the reference has #000000) |
| list-item-definition.html | step 2a-1 (generated content, not done) | uses generated content (6400 pixels differ (largest difference 255); first at (144,64): #000000, the reference has #ffffff) |
| list-style-type-decimal-vertical-lr.html | step 2a-1 (generated content, not done) | uses generated content (17600 pixels differ (largest difference 255); first at (8,40): #000000, the reference has #ffffff) |
| list-style-type-decimal-vertical-rl.html | step 2a-1 (generated content, not done) | uses generated content (17600 pixels differ (largest difference 255); first at (8,40): #000000, the reference has #ffffff) |
| list-style-type-string-001a.html | step 2a-1 (generated content, not done) | uses generated content (7168 pixels differ (largest difference 255); first at (64,16): #ffffff, the reference has #000000) |
| list-style-type-string-001b.html | step 2a-1 (generated content, not done) | uses generated content (7168 pixels differ (largest difference 255); first at (64,16): #ffffff, the reference has #000000) |
| list-style-type-string-002.html | step 2a-1 (generated content, not done) | uses generated content (4608 pixels differ (largest difference 255); first at (0,32): #000000, the reference has #ffffff) |
| list-style-type-string-003.html | step 2a-1 (generated content, not done) | uses generated content (4608 pixels differ (largest difference 255); first at (0,32): #000000, the reference has #ffffff) |
| list-style-type-string-005a.html | step 2a-1 (generated content, not done) | uses generated content (19456 pixels differ (largest difference 255); first at (200,12): #000000, the reference has #ffffff) |
| list-style-type-string-005b.html | step 2a-1 (generated content, not done) | uses generated content (48640 pixels differ (largest difference 255); first at (200,12): #000000, the reference has #ffffff) |
| list-style-type-string-006.html | step 2a-1 (generated content, not done) | uses generated content (23552 pixels differ (largest difference 255); first at (296,12): #000000, the reference has #ffffff) |
| list-style-type-string-007.html | step 2d | uses text features (5660 pixels differ (largest difference 255); first at (188,18): #ffffff, the reference has #000000) |
| list-type-none-style-image.html | step 2a-1 (generated content, not done) | uses generated content (256 pixels differ (largest difference 255); first at (16,16): #ffffff, the reference has #000000) |
| marker-counter.html | step 2a-1 (generated content, not done) | uses generated content (1536 pixels differ (largest difference 255); first at (184,96): #ffffff, the reference has #000000) |
| marker-webkit-text-fill-color.html | step 2a-1 (generated content, not done) | uses generated content (28835 pixels differ (largest difference 255); first at (8,13): #ff0000, the reference has #008000) |
| nested-marker-styling.html | step 2a-1 (generated content, not done) | uses generated content (9216 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #000000) |
| nested-marker.html | step 2a-1 (generated content, not done) | uses generated content (7680 pixels differ (largest difference 255); first at (96,16): #ffffff, the reference has #000000) |

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
- counter-invalid.htm
- counter-list-item-2.html
- counter-list-item-3.html
- counter-order-display-contents.html
- counter-reset-reversed-display-none.html
- counter-reset-reversed-multiple.html
- counter-reset-reversed-nested.html
- counter-reset-reversed-pseudo-001.html
- counter-reset-reversed-pseudo-002.html
- counter-reset-reversed-pseudo-003.html
- counter-reset-reversed-siblings-001a.html
- counter-reset-reversed-siblings-001b.html
- counter-reset-reversed-siblings-002.html
- counter-reset-reversed-siblings-003.html
- counter-reset-reversed-with-regular.html
- li-list-item-counter-002.html
- li-list-item-counter-003.html
- li-list-item-counter-004.html
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
- list-marker-alignment.html
- list-marker-counter-style-fallback-rtl.html
- list-marker-rtl-string-width.html
- list-marker-symbol-bidi.html
- list-marker-with-lineheight-and-overflow-hidden-001.html
- list-style-image-gradients.html
- list-style-position-001.html
- list-style-type-decimal-line-height.html
- marker-quotes.html
- outside-marker-covered-by-relpos-block-link.html
- outside-marker-in-multicol-relpos-content.html
- outside-marker-with-relpos-block-content.html
