# WPT `css/CSS2/floats-clear`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/CSS2/floats-clear`，用时 15.1 秒。

- 通过 **133 / 211**（63.0%）
- bug 类失败：**27**
- 跳过：needs JS 3，no reftest 35

失败分类：bug (open) 27，not planned 7，step 2a-1 (generated content, not done) 1，step 2a-2 36，step 2c 3，step 2e 4

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| adjoining-float-before-clearance.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (20000 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| adjoining-float-nested-forced-clearance-002.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (1200 pixels differ (largest difference 255); first at (8,42): #008000, the reference has #ffffff) |
| adjoining-float-nested-forced-clearance.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (20000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| adjoining-float-new-fc.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (40000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| clear-after-top-margin.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (9000 pixels differ (largest difference 255); first at (8,58): #ffffff, the reference has #008000) |
| clear-applies-to-001.xht | step 2a-2 | uses tables (98304 pixels differ (largest difference 255); first at (328,8): #ffffff, the reference has #0000ff) |
| clear-applies-to-002.xht | step 2a-2 | uses tables (98304 pixels differ (largest difference 255); first at (328,8): #ffffff, the reference has #0000ff) |
| clear-applies-to-003.xht | step 2a-2 | uses tables (98304 pixels differ (largest difference 255); first at (328,8): #ffffff, the reference has #0000ff) |
| clear-applies-to-004.xht | step 2a-2 | uses tables (129024 pixels differ (largest difference 255); first at (328,8): #ffffff, the reference has #0000ff) |
| clear-applies-to-005.xht | step 2a-2 | uses tables (18432 pixels differ (largest difference 255); first at (328,8): #ffffff, the reference has #0000ff) |
| clear-applies-to-006.xht | step 2a-2 | uses tables (18432 pixels differ (largest difference 255); first at (328,8): #ffffff, the reference has #0000ff) |
| clear-applies-to-007.xht | step 2a-2 | uses tables (18432 pixels differ (largest difference 255); first at (328,8): #ffffff, the reference has #0000ff) |
| clear-applies-to-013.xht | step 2a-2 | uses tables (27648 pixels differ (largest difference 255); first at (8,120): #ffffff, the reference has #0000ff) |
| clear-applies-to-014.xht | step 2a-2 | uses tables (9216 pixels differ (largest difference 255); first at (328,104): #0000ff, the reference has #ffffff) |
| clear-applies-to-015.xht | step 2a-2 | uses tables (18432 pixels differ (largest difference 255); first at (328,8): #ffffff, the reference has #0000ff) |
| clear-on-child-with-margins-2.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (2400 pixels differ (largest difference 255); first at (8,48): #ff0000, the reference has #008000) |
| clear-on-child-with-margins.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (300 pixels differ (largest difference 255); first at (58,48): #ffffff, the reference has #008000) |
| clear-on-parent-with-margins.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (25600 pixels differ (largest difference 255); first at (8,0): #008000, the reference has #ffffff) |
| clear-on-replaced-element.html | step 2c | uses painting effects (3942 pixels differ (largest difference 255); first at (13,89): #767676, the reference has #008000) |
| clear-with-top-margin-after-cleared-empty-block.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (2000 pixels differ (largest difference 255); first at (8,128): #ffffff, the reference has #008000) |
| float-applies-to-001.xht | step 2a-2 | uses tables (9216 pixels differ (largest difference 255); first at (696,64): #ffffff, the reference has #0000ff) |
| float-applies-to-002.xht | step 2a-2 | uses tables (9216 pixels differ (largest difference 255); first at (696,64): #ffffff, the reference has #0000ff) |
| float-applies-to-003.xht | step 2a-2 | uses tables (9216 pixels differ (largest difference 255); first at (696,64): #ffffff, the reference has #0000ff) |
| float-applies-to-004.xht | step 2a-2 | uses tables (9216 pixels differ (largest difference 255); first at (696,64): #ffffff, the reference has #0000ff) |
| float-applies-to-013.xht | step 2a-2 | uses tables (9216 pixels differ (largest difference 255); first at (696,160): #0000ff, the reference has #ffffff) |
| float-applies-to-014.xht | step 2a-2 | uses tables (9216 pixels differ (largest difference 255); first at (696,160): #0000ff, the reference has #ffffff) |
| float-replaced-height-004.xht | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation ...) (2432 pixels differ (largest difference 175); first at (11,51): #767676, the reference has #ffffff) |
| float-replaced-height-005.xht | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation ...) (3650 pixels differ (largest difference 255); first at (11,51): #767676, the reference has #ffffff) |
| float-replaced-height-006.xht | step 2e | uses svg, scrolling, sticky or form controls (19200 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #0000ff) |
| float-replaced-height-007.xht | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation ...) (2432 pixels differ (largest difference 175); first at (11,51): #767676, the reference has #ffffff) |
| float-replaced-width-007.xht | step 2e | uses svg, scrolling, sticky or form controls (10000 pixels differ (largest difference 255); first at (11,99): #ffffff, the reference has #0000ff) |
| float-replaced-width-008.xht | step 2e | uses svg, scrolling, sticky or form controls (20000 pixels differ (largest difference 255); first at (11,99): #ffffff, the reference has #0000ff) |
| float-replaced-width-009.xht | step 2e | uses svg, scrolling, sticky or form controls (20000 pixels differ (largest difference 255); first at (11,99): #ffffff, the reference has #0000ff) |
| floats-005.xht | bug (open) | not analysed one by one: 2a-1 ended at its tool-call budget; the cause is one of the families above or a separate float placement detail. Left open (3480 pixels differ (largest difference 255); first at (123,48): #ff0000, the reference has #008000) |
| floats-014.xht | step 2a-2 | uses tables (52024 pixels differ (largest difference 255); first at (250,80): #ffffff, the reference has #000000) |
| floats-015.xht | step 2a-2 | uses tables (52024 pixels differ (largest difference 255); first at (250,80): #ffffff, the reference has #000000) |
| floats-029.xht | bug (open) | not analysed one by one: 2a-1 ended at its tool-call budget; the cause is one of the families above or a separate float placement detail. Left open (56320 pixels differ (largest difference 255); first at (96,32): #000000, the reference has #ffffff) |
| floats-031.xht | bug (open) | not analysed one by one: 2a-1 ended at its tool-call budget; the cause is one of the families above or a separate float placement detail. Left open (23290 pixels differ (largest difference 255); first at (119,101): #000000, the reference has #ffffff) |
| floats-039.xht | bug (open) | not analysed one by one: 2a-1 ended at its tool-call budget; the cause is one of the families above or a separate float placement detail. Left open (3164 pixels differ (largest difference 255); first at (61,95): #ffa500, the reference has #ffffff) |
| floats-114.xht | step 2a-1 (generated content, not done) | uses generated content (1536 pixels differ (largest difference 255); first at (51,75): #ffffff, the reference has #000000) |
| floats-118.xht | bug (open) | not analysed one by one: 2a-1 ended at its tool-call budget; the cause is one of the families above or a separate float placement detail. Left open (2500 pixels differ (largest difference 255); first at (8,99): #ffffff, the reference has #000080) |
| floats-119.xht | bug (open) | not analysed one by one: 2a-1 ended at its tool-call budget; the cause is one of the families above or a separate float placement detail. Left open (11250 pixels differ (largest difference 255); first at (8,149): #ffffff, the reference has #000080) |
| floats-120.xht | bug (open) | not analysed one by one: 2a-1 ended at its tool-call budget; the cause is one of the families above or a separate float placement detail. Left open (11250 pixels differ (largest difference 255); first at (8,149): #ffffff, the reference has #000080) |
| floats-144.xht | bug (open) | not analysed one by one: 2a-1 ended at its tool-call budget; the cause is one of the families above or a separate float placement detail. Left open (15360 pixels differ (largest difference 255); first at (424,24): #ffffff, the reference has #008000) |
| floats-149.xht | step 2a-2 | uses tables (12544 pixels differ (largest difference 255); first at (400,8): #008000, the reference has #ffffff) |
| floats-150.xht | step 2a-2 | uses tables (5376 pixels differ (largest difference 255); first at (200,104): #ff0000, the reference has #ffffff) |
| floats-152.xht | step 2a-2 | uses tables (6400 pixels differ (largest difference 255); first at (200,104): #ff0000, the reference has #ffffff) |
| floats-153.xht | step 2a-2 | uses tables (2048 pixels differ (largest difference 255); first at (712,80): #008000, the reference has #ffffff) |
| floats-bfc-003.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (2500 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| floats-clear-multicol-000.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation ...) (1710 pixels differ (largest difference 255); first at (42,99): #ffa500, the reference has #ffffff) |
| floats-clear-multicol-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation ...) (1710 pixels differ (largest difference 255); first at (42,99): #ffa500, the reference has #ffffff) |
| floats-clear-multicol-balancing-000.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation ...) (2850 pixels differ (largest difference 255); first at (42,83): #ffa500, the reference has #ffffff) |
| floats-clear-multicol-balancing-001.html | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation ...) (2850 pixels differ (largest difference 255); first at (42,83): #ffa500, the reference has #ffffff) |
| margin-collapse-018.xht | step 2c | uses painting effects (4000 pixels differ (largest difference 255); first at (8,88): #008000, the reference has #ffffff) |
| margin-collapse-027.xht | step 2c | uses painting effects (4000 pixels differ (largest difference 255); first at (8,108): #008000, the reference has #ffffff) |
| margin-collapse-033.xht | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| margin-collapse-034.xht | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| margin-collapse-035.xht | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| margin-collapse-121.xht | step 2a-2 | uses tables (257912 pixels differ (largest difference 255); first at (8,48): #000000, the reference has #ffffff) |
| margin-collapse-122.xht | step 2a-2 | uses tables (269688 pixels differ (largest difference 255); first at (8,48): #000000, the reference has #ffffff) |
| margin-collapse-123.xht | step 2a-2 | uses tables (132052 pixels differ (largest difference 255); first at (8,48): #000000, the reference has #ffffff) |
| margin-collapse-125.xht | step 2a-2 | uses tables (207736 pixels differ (largest difference 255); first at (8,48): #000000, the reference has #ffffff) |
| margin-collapse-134.xht | step 2a-2 | uses tables (126900 pixels differ (largest difference 255); first at (8,64): #000000, the reference has #ffffff) |
| margin-collapse-142.xht | step 2a-2 | uses tables (402488 pixels differ (largest difference 255); first at (8,64): #000000, the reference has #ffffff) |
| margin-collapse-157.xht | step 2a-2 | uses tables (34352 pixels differ (largest difference 255); first at (12,204): #ff0000, the reference has #00ffff) |
| margin-collapse-158.xht | step 2a-2 | uses tables (179704 pixels differ (largest difference 255); first at (8,48): #000000, the reference has #ffffff) |
| margin-collapse-165.xht | step 2a-2 | uses tables (143856 pixels differ (largest difference 255); first at (126,64): #ff0000, the reference has #ffffff) |
| margin-collapse-clear-002.xht | step 2a-2 | uses tables (31777 pixels differ (largest difference 255); first at (14,64): #ffffff, the reference has #ffff00) |
| margin-collapse-clear-003.xht | step 2a-2 | uses tables (31817 pixels differ (largest difference 255); first at (14,64): #ffffff, the reference has #ffff00) |
| margin-collapse-clear-008.xht | step 2a-2 | uses tables (31777 pixels differ (largest difference 255); first at (14,64): #ffffff, the reference has #ffff00) |
| margin-collapse-clear-009.xht | step 2a-2 | uses tables (30837 pixels differ (largest difference 255); first at (14,64): #ffffff, the reference has #ffff00) |
| margin-collapse-clear-012.xht | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (138960 pixels differ (largest difference 255); first at (108,49): #ff0000, the reference has #00ff00) |
| margin-collapse-clear-013.xht | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (138960 pixels differ (largest difference 255); first at (108,49): #ff0000, the reference has #00ff00) |
| margin-collapse-clear-014.xht | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (68400 pixels differ (largest difference 255); first at (8,108): #ff0000, the reference has #00ff00) |
| negative-clearance-after-adjoining-float.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (15000 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| negative-clearance-after-bottom-margin.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (2550 pixels differ (largest difference 255); first at (58,48): #ffffff, the reference has #008000) |
| nested-clearance-new-formatting-context.html | bug (open) | margin collapsing through an empty block that holds a float, together with clearance (CSS2 8.3.1, 9.5.2, the 'adjoining float' rules browsers apply): the engine places the float after the following margins have collapsed in, and does not separate margins by clearance as browsers do; needs margin handling that is aware of floats. Not fixed in 2a-1 (17600 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| table-among-floats-001.html | step 2a-2 | uses tables (27500 pixels differ (largest difference 255); first at (133,64): #ff0000, the reference has #008000) |

## 通过

- adjacent-floats-001.xht
- adjoining-float-nested-forced-clearance-003.html
- adjoining-float-nested-forced-clearance-004.html
- clear-001.xht
- clear-002.xht
- clear-003.xht
- clear-004.xht
- clear-005.xht
- clear-applies-to-000.xht
- clear-applies-to-008.xht
- clear-applies-to-009.xht
- clear-applies-to-012.xht
- clear-clearance-calculation-001.xht
- clear-clearance-calculation-002.xht
- clear-clearance-calculation-003.xht
- clear-clearance-calculation-004.xht
- clear-clearance-calculation-005.xht
- clear-default-inheritance-001.xht
- clear-float-001.xht
- clear-float-002.xht
- clear-float-003.xht
- clear-float-004.xht
- clear-float-005.xht
- clear-float-006.xht
- clear-float-007.xht
- clear-float-008.xht
- clear-float-009.xht
- clear-initial-001.xht
- clear-inline-001.xht
- clear-on-parent-with-margins-no-clearance.html
- clear-on-parent.html
- clearance-006.xht
- float-003.xht
- float-005.xht
- float-006.xht
- float-applies-to-001a.xht
- float-applies-to-004a.xht
- float-applies-to-005.xht
- float-applies-to-006.xht
- float-applies-to-007.xht
- float-applies-to-008.xht
- float-applies-to-008a.xht
- float-applies-to-009.xht
- float-applies-to-012.xht
- float-applies-to-015.xht
- float-non-replaced-height-001.xht
- float-non-replaced-width-001.xht
- float-non-replaced-width-002.xht
- float-non-replaced-width-003.xht
- float-non-replaced-width-004.xht
- float-non-replaced-width-005.xht
- float-non-replaced-width-007.xht
- float-non-replaced-width-008.xht
- float-non-replaced-width-009.xht
- float-non-replaced-width-010.xht
- float-non-replaced-width-011.xht
- float-non-replaced-width-012.xht
- float-replaced-height-001.xht
- float-replaced-height-002.xht
- float-replaced-height-003.xht
- float-replaced-width-001.xht
- float-replaced-width-002.xht
- float-replaced-width-003.xht
- float-replaced-width-004.xht
- float-replaced-width-005.xht
- float-replaced-width-006.xht
- float-replaced-width-011.xht
- floats-001.xht
- floats-002.xht
- floats-003.xht
- floats-004.xht
- floats-006.xht
- floats-007.xht
- floats-008.xht
- floats-009.xht
- floats-019.xht
- floats-022.xht
- floats-023.xht
- floats-024.xht
- floats-025.xht
- floats-026.xht
- floats-027.xht
- floats-028.xht
- floats-030.xht
- floats-036.xht
- floats-038.xht
- floats-040.xht
- floats-041.xht
- floats-043.xht
- floats-101.xht
- floats-111.xht
- floats-112.xht
- floats-113.xht
- floats-115.xht
- floats-116.xht
- floats-117.xht
- floats-121.xht
- floats-122.xht
- floats-123.xht
- floats-124.xht
- floats-125.xht
- floats-132.xht
- floats-133.xht
- floats-134.xht
- floats-135.xht
- floats-136.xht
- floats-138.xht
- floats-139.xht
- floats-141.xht
- floats-142.xht
- floats-143.xht
- floats-145.xht
- floats-146.xht
- floats-147.xht
- floats-154.xht
- floats-bfc-001.xht
- floats-bfc-002.xht
- floats-clear-multicol-002.html
- floats-clear-multicol-003.html
- floats-clear-multicol-balancing-002.html
- floats-clear-multicol-balancing-003.html
- margin-collapse-023.xht
- margin-collapse-024.xht
- margin-collapse-031.xht
- margin-collapse-135.xht
- margin-collapse-clear-015.xht
- margin-collapse-clear-016.xht
- margin-collapse-clear-017.xht
- no-clearance-adjoining-opposite-float.html
- no-clearance-due-to-large-margin-after-left-right.html
- no-clearance-due-to-large-margin.html
- second-float-inside-empty-cleared-block-after-margin.html
- second-float-inside-empty-cleared-block.html
