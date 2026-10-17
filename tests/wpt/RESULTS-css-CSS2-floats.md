# WPT `css/CSS2/floats`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/CSS2/floats`，用时 0.6 秒。

- 通过 **76 / 100**（76.0%）
- bug 类失败：**22**
- 跳过：needs JS 8，no reftest 19

失败分类：bug (open) 22，step 2d 1，step 2e 1

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| float-in-inline-001.html | bug (open) | float inside an inline element with padding, border and margin: the float is placed relative to the line, the inline box edges are not added to its position (css-inline: the float sits at the line start edge but after the open inline box edges) (40000 pixels differ (largest difference 255); first at (208,64): #ff0000, the reference has #ffffff) |
| float-in-inline-002.html | bug (open) | same as float-in-inline-001 (float inside an inline element with padding, border and margin) (40000 pixels differ (largest difference 255); first at (208,64): #ff0000, the reference has #ffffff) |
| float-nowrap-7.html | bug (open) | white-space:nowrap line with a float after content: the line cannot wrap, so the float has to wait for the next line (css-text-3 white-space, CSS2 9.5.1 rule 4); the engine puts it beside the content on the same line (10478 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #000000) |
| float-nowrap-8.html | bug (open) | same as float-nowrap-7 with the float in the middle of a nowrap line (10478 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #000000) |
| floats-in-table-caption-001.html | bug (open) | used to be classed "2a-2 tables" (the page has a table); it still fails now that tables exist (first difference: (122,80): #ffffff, the reference has #000000); not analyzed one by one (3072 pixels differ (largest difference 255); first at (122,80): #ffffff, the reference has #000000) |
| floats-line-wrap-shifted-001.html | bug (open) | <nobr> content shifted by a float: the nowrap run that does not fit beside the float must move below it as a whole (css-text-3); same family as float-nowrap-7: the engine breaks the line before the nobr instead (3200 pixels differ (largest difference 255); first at (58,8): #000000, the reference has #ffff00) |
| floats-placement-008.html | bug (open) | static position of a block-level absolutely positioned box (display:block, left:auto) that follows an inline-block and floats in the same line: browsers put it at the container edge on a new line (x=8), the engine uses the inline position after the content (x=58); an out-of-flow positioning detail, not float placement (2500 pixels differ (largest difference 255); first at (8,114): #ff0000, the reference has #008000) |
| floats-wrap-bfc-005.xht | bug (open) | used to be classed "2a-2 tables" (the page has a table); it still fails now that tables exist (first difference: (208,8): #000000, the reference has #00ffff); not analyzed one by one (39000 pixels differ (largest difference 255); first at (208,8): #000000, the reference has #00ffff) |
| floats-wrap-bfc-006.xht | bug (open) | used to be classed "2a-2 tables" (the page has a table); it still fails now that tables exist (first difference: (218,122): #800080, the reference has #00ffff); not analyzed one by one (5460 pixels differ (largest difference 255); first at (218,122): #800080, the reference has #00ffff) |
| floats-wrap-bfc-with-margin-005.html | step 2d | uses text features (4000 pixels differ (largest difference 255); first at (8,124): #ff0000, the reference has #008000) |
| floats-wrap-bfc-with-margin-007.tentative.html | bug (open) | a block formatting context box beside floats whose margins or height decide where it fits (9.5 'must not overlap'): the position is chosen by the first width check and the margins are collapsed against the unclamped position; the box would have to be laid out at each candidate position with its margins collapsed there. Not fixed (10000 pixels differ (largest difference 255); first at (33,64): #ff0000, the reference has #008000) |
| floats-wrap-bfc-with-margin-008.tentative.html | bug (open) | a block formatting context box beside floats whose margins or height decide where it fits (9.5 'must not overlap'): the position is chosen by the first width check and the margins are collapsed against the unclamped position; the box would have to be laid out at each candidate position with its margins collapsed there. Not fixed (5000 pixels differ (largest difference 255); first at (8,114): #ffffff, the reference has #008000) |
| floats-wrap-bfc-with-margin-009.tentative.html | bug (open) | a block formatting context box beside floats whose margins or height decide where it fits (9.5 'must not overlap'): the position is chosen by the first width check and the margins are collapsed against the unclamped position; the box would have to be laid out at each candidate position with its margins collapsed there. Not fixed (5000 pixels differ (largest difference 255); first at (8,114): #ffffff, the reference has #008000) |
| floats-wrap-bfc-with-margin-010.html | bug (open) | a block formatting context box beside floats whose margins or height decide where it fits (9.5 'must not overlap'): the position is chosen by the first width check and the margins are collapsed against the unclamped position; the box would have to be laid out at each candidate position with its margins collapsed there. Not fixed (6250 pixels differ (largest difference 255); first at (333,63): #ffffff, the reference has #008000) |
| floats-wrap-top-below-bfc-001l.xht | bug (open) | a block formatting context box beside floats whose margins or height decide where it fits (9.5 'must not overlap'): the position is chosen by the first width check and the margins are collapsed against the unclamped position; the box would have to be laid out at each candidate position with its margins collapsed there. Not fixed (11950 pixels differ (largest difference 255); first at (111,61): #ffffff, the reference has #00ffff) |
| floats-wrap-top-below-bfc-001r.xht | bug (open) | a block formatting context box beside floats whose margins or height decide where it fits (9.5 'must not overlap'): the position is chosen by the first width check and the margins are collapsed against the unclamped position; the box would have to be laid out at each candidate position with its margins collapsed there. Not fixed (11950 pixels differ (largest difference 255); first at (111,61): #ffffff, the reference has #00ffff) |
| new-fc-relayout.html | bug (open) | a block formatting context box beside floats whose margins or height decide where it fits (9.5 'must not overlap'): the position is chosen by the first width check and the margins are collapsed against the unclamped position; the box would have to be laid out at each candidate position with its margins collapsed there. Not fixed (3750 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| new-fc-separates-from-float-2.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2; browsers resolve the float's position at the first float and separate margins by clearance): the engine collapses the following margins through the float's holder block and then applies clearance to the collapsed position. Tried 'a block that holds a float does not collapse through': it fixes new-fc-separates-from-float but breaks clear-clearance-calculation-001 to 005, which need the opposite, so it is not applied; the real fix is a float-aware margin strut (exclusion space) (40000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| new-fc-separates-from-float-3.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2; browsers resolve the float's position at the first float and separate margins by clearance): the engine collapses the following margins through the float's holder block and then applies clearance to the collapsed position. Tried 'a block that holds a float does not collapse through': it fixes new-fc-separates-from-float but breaks clear-clearance-calculation-001 to 005, which need the opposite, so it is not applied; the real fix is a float-aware margin strut (exclusion space) (18500 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| new-fc-separates-from-float-4.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2; browsers resolve the float's position at the first float and separate margins by clearance): the engine collapses the following margins through the float's holder block and then applies clearance to the collapsed position. Tried 'a block that holds a float does not collapse through': it fixes new-fc-separates-from-float but breaks clear-clearance-calculation-001 to 005, which need the opposite, so it is not applied; the real fix is a float-aware margin strut (exclusion space) (80000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| new-fc-separates-from-float.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2; browsers resolve the float's position at the first float and separate margins by clearance): the engine collapses the following margins through the float's holder block and then applies clearance to the collapsed position. Tried 'a block that holds a float does not collapse through': it fixes new-fc-separates-from-float but breaks clear-clearance-calculation-001 to 005, which need the opposite, so it is not applied; the real fix is a float-aware margin strut (exclusion space) (80000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| overflow-scroll-float-paint-order.html | step 2e | uses svg, scrolling, sticky or form controls (7200 pixels differ (largest difference 255); first at (28,108): #ffff00, the reference has #ffffff) |
| table-sizing-with-adjacent-floats.html | bug (open) | used to be classed "2a-2 tables" (the page has a table); it still fails now that tables exist (first difference: (8,64): #ffffff, the reference has #008000); not analyzed one by one (5000 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| zero-width-floats-positioning.tentative.html | bug (open) | a zero-width left float (50 high) beside a BFC box with margin-left:-50px: the engine ignores zero-width floats when it narrows the band (they intrude nowhere), the tentative test expects the BFC box to start at the float edge; BFC box x is -42 instead of 8 (10800 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |

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
- floated-table-wider-than-specified.html
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
- floats-placement-vertical-003.xht
- floats-placement-vertical-004.xht
- floats-rule3-outside-left-001.xht
- floats-rule3-outside-left-002.xht
- floats-rule3-outside-right-001.xht
- floats-rule3-outside-right-002.xht
- floats-rule7-outside-left-001.xht
- floats-rule7-outside-right-001.xht
- floats-wrap-bfc-001-left-overflow.xht
- floats-wrap-bfc-001-left-table.xht
- floats-wrap-bfc-001-right-overflow.xht
- floats-wrap-bfc-001-right-table.xht
- floats-wrap-bfc-002-left-overflow.xht
- floats-wrap-bfc-002-left-table.xht
- floats-wrap-bfc-002-right-overflow.xht
- floats-wrap-bfc-002-right-table.xht
- floats-wrap-bfc-003-left-overflow.xht
- floats-wrap-bfc-003-left-table.xht
- floats-wrap-bfc-003-right-overflow.xht
- floats-wrap-bfc-003-right-table.xht
- floats-wrap-bfc-004.xht
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
- new-fc-beside-float-with-margin.html
- new-fc-beside-float-with-min-width.html
- overhanging-float-paint-order.html
- zero-available-space-float-positioning.html
- zero-width-floats.html
