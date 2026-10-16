# WPT `css/CSS2/floats`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/CSS2/floats`，用时 7.4 秒。

- 通过 **66 / 100**（66.0%）
- bug 类失败：**22**
- 跳过：needs JS 8，no reftest 19

失败分类：bug (open) 22，step 2a-2 10，step 2d 1，step 2e 1

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| float-in-inline-001.html | bug (open) | float inside an inline element with padding, border and margin: the float is placed relative to the line but the inline box edges are not applied to its position. Not looked at further (40000 pixels differ (largest difference 255); first at (208,64): #ff0000, the reference has #ffffff) |
| float-in-inline-002.html | bug (open) | float inside an inline element with padding, border and margin: the float is placed relative to the line but the inline box edges are not applied to its position. Not looked at further (40000 pixels differ (largest difference 255); first at (208,64): #ff0000, the reference has #ffffff) |
| float-nowrap-7.html | bug (open) | white-space:nowrap lines and floats (a line that does not wrap puts its floats beside it; css-text-3 white-space) not handled: the float sits on the line instead of after it. Not looked at further (10478 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #000000) |
| float-nowrap-8.html | bug (open) | white-space:nowrap lines and floats (a line that does not wrap puts its floats beside it; css-text-3 white-space) not handled: the float sits on the line instead of after it. Not looked at further (10478 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #000000) |
| floated-table-wider-than-specified.html | step 2a-2 | uses tables (20500 pixels differ (largest difference 255); first at (218,64): #008000, the reference has #ffffff) |
| floats-in-table-caption-001.html | step 2a-2 | uses tables (3072 pixels differ (largest difference 255); first at (122,80): #ffffff, the reference has #000000) |
| floats-line-wrap-shifted-001.html | bug (open) | not analysed one by one: 2a-1 ended at its tool-call budget; the cause is one of the families above or a separate float placement detail. Left open (3200 pixels differ (largest difference 255); first at (58,8): #000000, the reference has #ffff00) |
| floats-placement-008.html | bug (open) | not analysed one by one: 2a-1 ended at its tool-call budget; the cause is one of the families above or a separate float placement detail. Left open (2500 pixels differ (largest difference 255); first at (8,114): #ff0000, the reference has #008000) |
| floats-placement-vertical-003.xht | bug (open) | not analysed one by one: 2a-1 ended at its tool-call budget; the cause is one of the families above or a separate float placement detail. Left open (5625 pixels differ (largest difference 255); first at (8,8): #ffff00, the reference has #000000) |
| floats-rule3-outside-left-001.xht | bug (open) | not analysed one by one: 2a-1 ended at its tool-call budget; the cause is one of the families above or a separate float placement detail. Left open (8500 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #0000ff) |
| floats-rule3-outside-right-001.xht | bug (open) | not analysed one by one: 2a-1 ended at its tool-call budget; the cause is one of the families above or a separate float placement detail. Left open (8500 pixels differ (largest difference 255); first at (83,8): #ffffff, the reference has #0000ff) |
| floats-wrap-bfc-001-left-table.xht | step 2a-2 | uses tables (2500 pixels differ (largest difference 255); first at (258,8): #ffff00, the reference has #00ffff) |
| floats-wrap-bfc-001-right-table.xht | step 2a-2 | uses tables (2500 pixels differ (largest difference 255); first at (158,8): #ffff00, the reference has #00ffff) |
| floats-wrap-bfc-003-left-table.xht | step 2a-2 | uses tables (27500 pixels differ (largest difference 255); first at (108,8): #800080, the reference has #00ffff) |
| floats-wrap-bfc-003-right-table.xht | step 2a-2 | uses tables (25000 pixels differ (largest difference 255); first at (8,8): #800080, the reference has #00ffff) |
| floats-wrap-bfc-004.xht | step 2a-2 | uses tables (7700 pixels differ (largest difference 255); first at (208,8): #800080, the reference has #00ffff) |
| floats-wrap-bfc-005.xht | step 2a-2 | uses tables (39000 pixels differ (largest difference 255); first at (208,8): #000000, the reference has #00ffff) |
| floats-wrap-bfc-006.xht | step 2a-2 | uses tables (69578 pixels differ (largest difference 255); first at (158,8): #000000, the reference has #00ffff) |
| floats-wrap-bfc-with-margin-005.html | step 2d | uses text features (4000 pixels differ (largest difference 255); first at (8,124): #ff0000, the reference has #008000) |
| floats-wrap-bfc-with-margin-007.tentative.html | bug (open) | a block formatting context box next to floats whose margins or height decide where it fits (9.5 'must not overlap'): placed by the first width/height check only; the remaining cases need the box laid out at each candidate position with its margins collapsed. Not fixed in 2a-1 (10000 pixels differ (largest difference 255); first at (33,64): #ff0000, the reference has #008000) |
| floats-wrap-bfc-with-margin-008.tentative.html | bug (open) | a block formatting context box next to floats whose margins or height decide where it fits (9.5 'must not overlap'): placed by the first width/height check only; the remaining cases need the box laid out at each candidate position with its margins collapsed. Not fixed in 2a-1 (5000 pixels differ (largest difference 255); first at (8,114): #ffffff, the reference has #008000) |
| floats-wrap-bfc-with-margin-009.tentative.html | bug (open) | a block formatting context box next to floats whose margins or height decide where it fits (9.5 'must not overlap'): placed by the first width/height check only; the remaining cases need the box laid out at each candidate position with its margins collapsed. Not fixed in 2a-1 (5000 pixels differ (largest difference 255); first at (8,114): #ffffff, the reference has #008000) |
| floats-wrap-bfc-with-margin-010.html | bug (open) | a block formatting context box next to floats whose margins or height decide where it fits (9.5 'must not overlap'): placed by the first width/height check only; the remaining cases need the box laid out at each candidate position with its margins collapsed. Not fixed in 2a-1 (6250 pixels differ (largest difference 255); first at (333,63): #ffffff, the reference has #008000) |
| floats-wrap-top-below-bfc-001l.xht | bug (open) | a block formatting context box next to floats whose margins or height decide where it fits (9.5 'must not overlap'): placed by the first width/height check only; the remaining cases need the box laid out at each candidate position with its margins collapsed. Not fixed in 2a-1 (11950 pixels differ (largest difference 255); first at (111,61): #ffffff, the reference has #00ffff) |
| floats-wrap-top-below-bfc-001r.xht | bug (open) | a block formatting context box next to floats whose margins or height decide where it fits (9.5 'must not overlap'): placed by the first width/height check only; the remaining cases need the box laid out at each candidate position with its margins collapsed. Not fixed in 2a-1 (11950 pixels differ (largest difference 255); first at (111,61): #ffffff, the reference has #00ffff) |
| new-fc-beside-float-with-margin.html | bug (open) | a block formatting context box next to floats whose margins or height decide where it fits (9.5 'must not overlap'): placed by the first width/height check only; the remaining cases need the box laid out at each candidate position with its margins collapsed. Not fixed in 2a-1 (10000 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| new-fc-relayout.html | bug (open) | a block formatting context box next to floats whose margins or height decide where it fits (9.5 'must not overlap'): placed by the first width/height check only; the remaining cases need the box laid out at each candidate position with its margins collapsed. Not fixed in 2a-1 (3750 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| new-fc-separates-from-float-2.html | bug (open) | a block formatting context box next to floats whose margins or height decide where it fits (9.5 'must not overlap'): placed by the first width/height check only; the remaining cases need the box laid out at each candidate position with its margins collapsed. Not fixed in 2a-1 (40000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| new-fc-separates-from-float-3.html | bug (open) | a block formatting context box next to floats whose margins or height decide where it fits (9.5 'must not overlap'): placed by the first width/height check only; the remaining cases need the box laid out at each candidate position with its margins collapsed. Not fixed in 2a-1 (18500 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| new-fc-separates-from-float-4.html | bug (open) | a block formatting context box next to floats whose margins or height decide where it fits (9.5 'must not overlap'): placed by the first width/height check only; the remaining cases need the box laid out at each candidate position with its margins collapsed. Not fixed in 2a-1 (80000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| new-fc-separates-from-float.html | bug (open) | a block formatting context box next to floats whose margins or height decide where it fits (9.5 'must not overlap'): placed by the first width/height check only; the remaining cases need the box laid out at each candidate position with its margins collapsed. Not fixed in 2a-1 (80000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| overflow-scroll-float-paint-order.html | step 2e | uses svg, scrolling, sticky or form controls (7200 pixels differ (largest difference 255); first at (28,108): #ffff00, the reference has #ffffff) |
| table-sizing-with-adjacent-floats.html | step 2a-2 | uses tables (5000 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| zero-width-floats-positioning.tentative.html | bug (open) | not analysed one by one: 2a-1 ended at its tool-call budget; the cause is one of the families above or a separate float placement detail. Left open (10800 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |

## 通过

- float-after-block-with-collapsed-margin-inside-inline.html
- float-in-inline-anonymous-block-with-overflow-hidden.html
- float-in-nested-multicol-001.html
- float-no-content-beside-001.html
- float-nowrap-1.html
- float-nowrap-2.html
- float-nowrap-3.html
- float-nowrap-4.html
- float-nowrap-5.html
- float-nowrap-6.html
- float-nowrap-9.html
- float-nowrap-hyphen-rewind-1.html
- float-root.html
- float-table-align-left-quirk.html
- float-under-flatten-under-preserve-3d.html
- float-with-absolutely-positioned-child-with-static-inset.html
- floats-placement-001.html
- floats-placement-002.html
- floats-placement-003.html
- floats-placement-004.html
- floats-placement-005.html
- floats-placement-006.html
- floats-placement-007.html
- floats-placement-vertical-001a.xht
- floats-placement-vertical-001b.xht
- floats-placement-vertical-001c.xht
- floats-placement-vertical-004.xht
- floats-rule3-outside-left-002.xht
- floats-rule3-outside-right-002.xht
- floats-rule7-outside-left-001.xht
- floats-rule7-outside-right-001.xht
- floats-wrap-bfc-001-left-overflow.xht
- floats-wrap-bfc-001-right-overflow.xht
- floats-wrap-bfc-002-left-overflow.xht
- floats-wrap-bfc-002-left-table.xht
- floats-wrap-bfc-002-right-overflow.xht
- floats-wrap-bfc-002-right-table.xht
- floats-wrap-bfc-003-left-overflow.xht
- floats-wrap-bfc-003-right-overflow.xht
- floats-wrap-bfc-007.xht
- floats-wrap-bfc-008.html
- floats-wrap-bfc-outside-001.xht
- floats-wrap-bfc-with-margin-004.html
- floats-wrap-bfc-with-margin-006.tentative.html
- floats-wrap-top-below-bfc-002l.xht
- floats-wrap-top-below-bfc-002r.xht
- floats-wrap-top-below-bfc-003l.xht
- floats-wrap-top-below-bfc-003r.xht
- floats-wrap-top-below-inline-001l.xht
- floats-wrap-top-below-inline-001r.xht
- floats-wrap-top-below-inline-002l.xht
- floats-wrap-top-below-inline-002r.xht
- floats-wrap-top-below-inline-003l.xht
- floats-wrap-top-below-inline-003r.xht
- floats-zero-height-wrap-001.xht
- floats-zero-height-wrap-002.xht
- intrinsic-size-float-and-line.html
- negative-block-margin-pushing-float-out-of-block-formatting-context.html
- negative-margin-float-positioning.html
- new-fc-beside-adjoining-float-2.html
- new-fc-beside-adjoining-float.html
- new-fc-beside-float-with-margin-rtl.html
- new-fc-beside-float-with-min-width.html
- overhanging-float-paint-order.html
- zero-available-space-float-positioning.html
- zero-width-floats.html
