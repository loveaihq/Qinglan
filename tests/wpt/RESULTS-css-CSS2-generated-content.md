# WPT `css/CSS2/generated-content`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/CSS2/generated-content`，用时 1.2 秒。

- 通过 **188 / 205**（91.7%）
- bug 类失败：**3**
- 跳过：needs JS 21，no reftest 90

失败分类：bug (open) 3，limit (counter styles) 2，limit (css-lists-3 counter scope) 3，limit (test platform) 3，limit (xhtml as html) 2，not planned 1，step 2a-2 1，step 2d 2

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| before-after-table-parts-001.xht | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10466 pixels differ (largest difference 255); first at (104,290): #0000ff, the reference has #ffffff) |
| before-after-table-whitespace-001.xht | bug (open) | used to be classed "2a-2 tables" (the page has a table); it still fails now that tables exist (first difference: (17,61): #000000, the reference has #ffffff); not analyzed one by one (5120 pixels differ (largest difference 255); first at (17,61): #000000, the reference has #ffffff) |
| bidi-generated-content-001.xht | step 2d | unicode-bidi/direction in generated content: bidi is step 2d (256 pixels differ (largest difference 255); first at (520,80): #000000, the reference has #ffffff) |
| bidi-generated-content-002.xht | step 2d | unicode-bidi/direction in generated content: bidi is step 2d (768 pixels differ (largest difference 255); first at (104,80): #000000, the reference has #ffffff) |
| content-082.xht | step 2a-2 | generated content in a table cell (td:before); tables are step 2a-2 (16 pixels differ (largest difference 238); first at (10,81): #0000ee, the reference has #008000) |
| content-174.xht | limit (test platform) | the generated text (white-space:nowrap, 53 glyphs of monospace 16px) is 848 px wide in the test platform, the reference wraps at the 800 px viewport; with a real monospace font it fits (256 pixels differ (largest difference 255); first at (648,8): #008000, the reference has #ffffff) |
| content-175.xht | bug (open) | an inline box whose ::after ends in a preserved newline (white-space:pre, "  \A"): after the forced break the closing padding of the inline box goes to a new empty line (layout shows a third line box holding only the [last] inline fragment); browsers keep it on the broken line and make no extra line (5888 pixels differ (largest difference 255); first at (376,72): #ffffff, the reference has #000080) |
| content-attr-case-002.xht | limit (xhtml as html) | XHTML test: attr(Title) is case-sensitive in XML and finds no attribute; the harness parses .xht as HTML, where attribute names are lower-cased and attr() lower-cases its argument (1024 pixels differ (largest difference 255); first at (8,64): #ffff00, the reference has #ffffff) |
| content-counter-004.xht | bug (open) | counter(c, square) as text "▪" against a reference made of display:list-item markers (list-style-position:inside, "▪ " with suffix): positions differ from the first marker line (8,64); not isolated: marker versus counter text spacing in the test platform (3072 pixels differ (largest difference 255); first at (8,64): #0000ff, the reference has #ffffff) |
| content-counter-006.xht | limit (css-lists-3 counter scope) | CSS2.1 expects a counter reset on a span to be seen by its following siblings; css-lists-3 4.4.1 (and browsers, css-lists/counter-001 is the same page with the new expectation) keep the parent counter of the same name; the engine follows css-lists-3 (1024 pixels differ (largest difference 255); first at (664,48): #ffffff, the reference has #000000) |
| content-counter-007.xht | limit (css-lists-3 counter scope) | CSS2.1 expects a counter reset on a span to be seen by its following siblings; css-lists-3 4.4.1 (and browsers, css-lists/counter-001 is the same page with the new expectation) keep the parent counter of the same name; the engine follows css-lists-3 (512 pixels differ (largest difference 255); first at (712,48): #000000, the reference has #ffffff) |
| content-counter-008.xht | limit (css-lists-3 counter scope) | CSS2.1 expects a counter reset on a span to be seen by its following siblings; css-lists-3 4.4.1 (and browsers, css-lists/counter-001 is the same page with the new expectation) keep the parent counter of the same name; the engine follows css-lists-3 (512 pixels differ (largest difference 255); first at (712,48): #000000, the reference has #ffffff) |
| content-counter-009.xht | limit (counter styles) | counter(c, georgian): the Georgian numbering system is not implemented, unknown counter styles fall back to decimal (4224 pixels differ (largest difference 255); first at (312,48): #000000, the reference has #ffffff) |
| content-counter-010.xht | limit (counter styles) | counter(c, armenian): the Armenian numbering system is not implemented, unknown counter styles fall back to decimal (4224 pixels differ (largest difference 255); first at (312,48): #000000, the reference has #ffffff) |
| content-newline-001.xht | limit (test platform) | white-space:pre "This text\Ashould be on two lines." in a 200 px box: 23 glyphs of 16 px do not fit in the test platform, the reference wraps; with a real font it fits (4608 pixels differ (largest difference 255); first at (216,80): #000000, the reference has #ffffff) |
| content-white-space-002.xht | limit (test platform) | a tab in white-space:pre text draws as a 1em square in the test platform (every glyph is a square); the reference uses 8 spaces (11264 pixels differ (largest difference 255); first at (22,86): #0000ff, the reference has #ffffff) |
| quotes-036.xht | limit (xhtml as html) | the test writes <div/> (XHTML empty-element tags), which the HTML parser reads as open tags: the nesting differs from the XML one the test needs (15616 pixels differ (largest difference 255); first at (184,8): #ffffff, the reference has #000000) |

## 通过

- after-content-display-001.xht
- after-content-display-002.xht
- after-content-display-003.xht
- after-content-display-005.xht
- after-content-display-006.xht
- after-content-display-007.xht
- after-content-display-008.xht
- after-content-display-009.xht
- after-content-display-010.xht
- after-content-display-011.xht
- after-content-display-012.xht
- after-content-display-013.xht
- after-content-display-014.xht
- after-content-display-015.xht
- after-content-display-016.xht
- after-content-display-017.xht
- after-content-display-018.xht
- after-inheritable-001.xht
- after-inheritable-002.xht
- after-location-001.xht
- before-after-001.xht
- before-after-002.xht
- before-after-011.xht
- before-after-display-types-001.xht
- before-after-floated-001.xht
- before-after-images-001.xht
- before-after-positioned-001.xht
- before-after-positioned-002.html
- before-after-positioned-003.html
- before-after-positioned-004.html
- before-content-display-001.xht
- before-content-display-002.xht
- before-content-display-003.xht
- before-content-display-005.xht
- before-content-display-006.xht
- before-content-display-007.xht
- before-content-display-008.xht
- before-content-display-009.xht
- before-content-display-010.xht
- before-content-display-011.xht
- before-content-display-012.xht
- before-content-display-013.xht
- before-content-display-014.xht
- before-content-display-015.xht
- before-content-display-016.xht
- before-content-display-017.xht
- before-content-display-018.xht
- before-inheritable-001.xht
- before-inheritable-002.xht
- before-location-001.xht
- content-001.xht
- content-002.xht
- content-003.xht
- content-004.xht
- content-005.xht
- content-006.xht
- content-007.xht
- content-009.xht
- content-010.xht
- content-011.xht
- content-012.xht
- content-013.xht
- content-014.xht
- content-015.xht
- content-016.xht
- content-017.xht
- content-018.xht
- content-019.xht
- content-020.xht
- content-021.xht
- content-022.xht
- content-023.xht
- content-025.xht
- content-026.xht
- content-027.xht
- content-028.xht
- content-029.xht
- content-030.xht
- content-031.xht
- content-032.xht
- content-033.xht
- content-034.xht
- content-035.xht
- content-036.xht
- content-037.xht
- content-038.xht
- content-039.xht
- content-040.xht
- content-041.xht
- content-042.xht
- content-043.xht
- content-046.xht
- content-047.xht
- content-048.xht
- content-050.xht
- content-052.xht
- content-053.xht
- content-054.xht
- content-056.xht
- content-057.xht
- content-063.xht
- content-065.xht
- content-067.xht
- content-068.xht
- content-070.xht
- content-073.xht
- content-075.xht
- content-076.xht
- content-077.xht
- content-080.xht
- content-081.xht
- content-083.xht
- content-085.xht
- content-086.xht
- content-089.xht
- content-091.xht
- content-096.xht
- content-097.xht
- content-099.xht
- content-100.xht
- content-103.xht
- content-123.xht
- content-126.xht
- content-127.xht
- content-129.xht
- content-131.xht
- content-132.xht
- content-135.xht
- content-136.xht
- content-141.xht
- content-143.xht
- content-144.xht
- content-145.xht
- content-146.xht
- content-147.xht
- content-149.xht
- content-150.xht
- content-152.xht
- content-153.xht
- content-155.xht
- content-156.xht
- content-157.xht
- content-158.xht
- content-159.xht
- content-171.xht
- content-172.xht
- content-173.xht
- content-177.xht
- content-attr-001.xht
- content-attr-case-001.html
- content-auto-reset-001.xht
- content-counter-000.xht
- content-counter-001.xht
- content-counter-002.xht
- content-counter-003.xht
- content-counter-005.xht
- content-counter-011.xht
- content-counter-012.xht
- content-counter-013.xht
- content-counter-014.xht
- content-counter-015.xht
- content-counter-016.xht
- content-inherit-002.xht
- content-white-space-001.xht
- content-white-space-003.xht
- content-white-space-004.xht
- counters-hidden-000.xht
- counters-hidden-001.xht
- counters-hidden-002.xht
- counters-multi-000.xht
- counters-multi-001.xht
- counters-order-000.xht
- counters-root-000.xht
- multiple-content-values-001.xht
- other-attribute-001.xht
- quotes-035.xht
- quotes-035a.xht
- quotes-applies-to-001.xht
- quotes-applies-to-002.xht
- quotes-applies-to-003.xht
- quotes-applies-to-004.xht
- quotes-applies-to-007.xht
- quotes-applies-to-008.xht
- quotes-applies-to-009.xht
- quotes-applies-to-012.xht
- quotes-applies-to-013.xht
- quotes-applies-to-014.xht
- quotes-applies-to-015.xht
