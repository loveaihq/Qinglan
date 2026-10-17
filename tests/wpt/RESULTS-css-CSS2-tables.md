# WPT `css/CSS2/tables`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/CSS2/tables`，用时 1.3 秒。

- 通过 **219 / 250**（87.6%）
- bug 类失败：**26**
- 跳过：无

失败分类：bug (open) 26，not planned 1，step 2c 4

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| border-collapse-offset-001.xht | bug (open) | collapsed borders: col / colgroup borders are not part of the conflict resolution, positioned rows / cells, spanning cells with different edge widths, double style (2c); not analyzed one by one (592 pixels differ (largest difference 255); first at (122,8): #ffffff, the reference has #008000) |
| border-collapse-visibility-collapse-001.tentative.html | bug (open) | visibility: collapse is done for rows (the row and its spacing go); columns, and cells that span collapsed rows or columns, are not (5000 pixels differ (largest difference 255); first at (108,64): #008000, the reference has #ffffff) |
| border-conflict-element-001d.xht | bug (open) | collapsed borders: col / colgroup borders are not part of the conflict resolution, positioned rows / cells, spanning cells with different edge widths, double style (2c); not analyzed one by one (6000 pixels differ (largest difference 255); first at (48,104): #ffff00, the reference has #0000ff) |
| collapsing-border-model-013.html | bug (open) | collapsed borders: col / colgroup borders are not part of the conflict resolution, positioned rows / cells, spanning cells with different edge widths, double style (2c); not analyzed one by one (20000 pixels differ (largest difference 255); first at (108,48): #008000, the reference has #ffffff) |
| collapsing-border-model-014.html | bug (open) | collapsed borders: col / colgroup borders are not part of the conflict resolution, positioned rows / cells, spanning cells with different edge widths, double style (2c); not analyzed one by one (10000 pixels differ (largest difference 255); first at (108,48): #008000, the reference has #ffffff) |
| column-visibility-004.xht | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation ...) (30000 pixels differ (largest difference 255); first at (112,66): #ff0000, the reference has #008000) |
| empty-cells-applies-to-011.xht | bug (open) | a row whose cells are all empty with empty-cells: hide has no height (CSS2 17.6.1.1); not done (60 pixels differ (largest difference 255); first at (3,69): #ff0000, the reference has #ffffff) |
| empty-cells-applies-to-012.xht | bug (open) | a row whose cells are all empty with empty-cells: hide has no height (CSS2 17.6.1.1); not done (128 pixels differ (largest difference 255); first at (8,72): #ff0000, the reference has #ffffff) |
| empty-cells-applies-to-017.xht | bug (open) | a row whose cells are all empty with empty-cells: hide has no height (CSS2 17.6.1.1); not done (60 pixels differ (largest difference 255); first at (3,69): #ff0000, the reference has #ffffff) |
| fixed-table-layout-017.xht | bug (open) | fixed layout with col / colgroup elements (width in %, span, widths on the group): not analyzed one by one (1280 pixels differ (largest difference 255); first at (180,80): #ffa500, the reference has #ffffff) |
| fixed-table-layout-018.xht | bug (open) | fixed layout with col / colgroup elements (width in %, span, widths on the group): not analyzed one by one (1280 pixels differ (largest difference 255); first at (180,80): #ffa500, the reference has #ffffff) |
| fixed-table-layout-019.xht | bug (open) | fixed layout with col / colgroup elements (width in %, span, widths on the group): not analyzed one by one (1280 pixels differ (largest difference 255); first at (180,80): #ffa500, the reference has #ffffff) |
| fixed-table-layout-020.xht | bug (open) | fixed layout with col / colgroup elements (width in %, span, widths on the group): not analyzed one by one (1280 pixels differ (largest difference 255); first at (180,80): #ffa500, the reference has #ffffff) |
| fixed-table-layout-021.xht | bug (open) | fixed layout with col / colgroup elements (width in %, span, widths on the group): not analyzed one by one (2112 pixels differ (largest difference 255); first at (52,80): #ff00ff, the reference has #ffffff) |
| fixed-table-layout-022.xht | bug (open) | fixed layout with col / colgroup elements (width in %, span, widths on the group): not analyzed one by one (3200 pixels differ (largest difference 255); first at (52,80): #ff00ff, the reference has #ffffff) |
| fixed-table-layout-023.xht | bug (open) | fixed layout with col / colgroup elements (width in %, span, widths on the group): not analyzed one by one (3200 pixels differ (largest difference 255); first at (37,96): #ff00ff, the reference has #ffffff) |
| fixed-table-layout-027.xht | bug (open) | fixed layout with col / colgroup elements (width in %, span, widths on the group): not analyzed one by one (200 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| table-anonymous-objects-079.xht | bug (open) | font-family: monospace with white-space: nowrap on body: the text of the red and green parts differs on the test platform; not analyzed (76869 pixels differ (largest difference 255); first at (710,13): #000000, the reference has #ffffff) |
| table-anonymous-objects-080.xht | bug (open) | font-family: monospace with white-space: nowrap on body: the text of the red and green parts differs on the test platform; not analyzed (75985 pixels differ (largest difference 255); first at (710,13): #000000, the reference has #ffffff) |
| table-anonymous-objects-081.xht | bug (open) | font-family: monospace with white-space: nowrap on body: the text of the red and green parts differs on the test platform; not analyzed (75023 pixels differ (largest difference 255); first at (710,13): #000000, the reference has #ffffff) |
| table-anonymous-objects-082.xht | bug (open) | font-family: monospace with white-space: nowrap on body: the text of the red and green parts differs on the test platform; not analyzed (75023 pixels differ (largest difference 255); first at (710,13): #000000, the reference has #ffffff) |
| table-anonymous-objects-083.xht | bug (open) | font-family: monospace with white-space: nowrap on body: the text of the red and green parts differs on the test platform; not analyzed (75023 pixels differ (largest difference 255); first at (710,13): #000000, the reference has #ffffff) |
| table-anonymous-objects-084.xht | bug (open) | font-family: monospace with white-space: nowrap on body: the text of the red and green parts differs on the test platform; not analyzed (75023 pixels differ (largest difference 255); first at (710,13): #000000, the reference has #ffffff) |
| table-anonymous-objects-085.xht | bug (open) | font-family: monospace with white-space: nowrap on body: the text of the red and green parts differs on the test platform; not analyzed (75985 pixels differ (largest difference 255); first at (710,13): #000000, the reference has #ffffff) |
| table-anonymous-objects-086.xht | bug (open) | font-family: monospace with white-space: nowrap on body: the text of the red and green parts differs on the test platform; not analyzed (76869 pixels differ (largest difference 255); first at (710,13): #000000, the reference has #ffffff) |
| table-anonymous-objects-211.xht | bug (open) | not analyzed (3600 pixels differ (largest difference 255); first at (120,80): #ffffff, the reference has #0000ff) |
| table-backgrounds-bs-colgroup-001.xht | step 2c | uses painting effects (3678 pixels differ (largest difference 255); first at (91,22): #00ffff, the reference has #ffffff) |
| table-backgrounds-bs-column-001.xht | step 2c | uses painting effects (1062 pixels differ (largest difference 255); first at (152,22): #00ffff, the reference has #ffffff) |
| table-backgrounds-bs-row-001.xht | step 2c | uses painting effects (168 pixels differ (largest difference 255); first at (89,73): #00ffff, the reference has #ffffff) |
| table-backgrounds-bs-rowgroup-001.xht | step 2c | uses painting effects (2322 pixels differ (largest difference 255); first at (89,49): #00ffff, the reference has #ffffff) |
| table-vertical-align-baseline-009.xht | bug (open) | baseline details of table cells and inline-tables (empty cells, absolutely positioned boxes in a cell); not analyzed (10000 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |

## 通过

- anonymous-table-box-width-001.xht
- border-collapse-005.html
- border-collapse-006.html
- border-collapse-applies-to-016.xht
- border-collapse-empty-row.html
- border-collapse-offset-002.xht
- border-collapse-visibility-collapse-002.tentative.html
- border-conflict-element-001a.xht
- border-conflict-element-001b.xht
- border-conflict-element-001c.xht
- border-conflict-element-001e.xht
- border-spacing-095.html
- border-spacing-applies-to-016.xht
- border-spacing-percentage-001.xht
- caption-position-001.xht
- caption-side-applies-to-001.xht
- caption-side-applies-to-002.xht
- caption-side-applies-to-003.xht
- caption-side-applies-to-005.xht
- caption-side-applies-to-006.xht
- caption-side-applies-to-007.xht
- caption-side-applies-to-008.xht
- caption-side-applies-to-009.xht
- caption-side-applies-to-010.xht
- caption-side-applies-to-011.xht
- caption-side-applies-to-012.xht
- caption-side-applies-to-013.xht
- caption-side-applies-to-014.xht
- caption-side-applies-to-015.xht
- caption-side-applies-to-016.xht
- caption-side-applies-to-017.xht
- collapsing-border-model-001.xht
- collapsing-border-model-003.xht
- collapsing-border-model-004.xht
- collapsing-border-model-007.xht
- collapsing-border-model-008.xht
- collapsing-border-model-009.xht
- collapsing-border-model-010a.xht
- collapsing-border-model-010b.xht
- collapsing-border-model-011.html
- collapsing-border-model-012.html
- empty-cells-applies-to-008.xht
- empty-cells-applies-to-009.xht
- empty-cells-applies-to-010.xht
- empty-cells-applies-to-013.xht
- empty-cells-applies-to-014.xht
- empty-cells-applies-to-016.xht
- fixed-table-layout-002a.xht
- fixed-table-layout-003a01.xht
- fixed-table-layout-003a02.xht
- fixed-table-layout-003a03.xht
- fixed-table-layout-003a04.xht
- fixed-table-layout-003a05.xht
- fixed-table-layout-003a06.xht
- fixed-table-layout-003b01.xht
- fixed-table-layout-003b02.xht
- fixed-table-layout-003b03.xht
- fixed-table-layout-003b04.xht
- fixed-table-layout-003b05.xht
- fixed-table-layout-003b06.xht
- fixed-table-layout-003b07.xht
- fixed-table-layout-003b08.xht
- fixed-table-layout-003b09.xht
- fixed-table-layout-003b10.xht
- fixed-table-layout-003b11.xht
- fixed-table-layout-003b12.xht
- fixed-table-layout-003c01.xht
- fixed-table-layout-003c02.xht
- fixed-table-layout-003c03.xht
- fixed-table-layout-003c04.xht
- fixed-table-layout-003c05.xht
- fixed-table-layout-003c06.xht
- fixed-table-layout-003c07.xht
- fixed-table-layout-003c08.xht
- fixed-table-layout-003d01.xht
- fixed-table-layout-003d02.xht
- fixed-table-layout-003d03.xht
- fixed-table-layout-003d04.xht
- fixed-table-layout-003d05.xht
- fixed-table-layout-003d06.xht
- fixed-table-layout-003e01.xht
- fixed-table-layout-003e02.xht
- fixed-table-layout-003e03.xht
- fixed-table-layout-003e04.xht
- fixed-table-layout-003e05.xht
- fixed-table-layout-003e06.xht
- fixed-table-layout-003e07.xht
- fixed-table-layout-003e08.xht
- fixed-table-layout-003e09.xht
- fixed-table-layout-003e10.xht
- fixed-table-layout-003e11.xht
- fixed-table-layout-003e12.xht
- fixed-table-layout-003f01.xht
- fixed-table-layout-003f02.xht
- fixed-table-layout-003f03.xht
- fixed-table-layout-003f04.xht
- fixed-table-layout-003f05.xht
- fixed-table-layout-003f06.xht
- fixed-table-layout-003f07.xht
- fixed-table-layout-003f08.xht
- fixed-table-layout-025.xht
- fixed-table-layout-026.xht
- fixed-table-layout-028.xht
- fixed-table-layout-029.xht
- fixed-table-layout-030.xht
- fixed-table-layout-031.xht
- height-table-cell-001.xht
- height-width-inline-table-001.xht
- height-width-inline-table-001a.xht
- height-width-inline-table-001b.xht
- height-width-inline-table-001c.xht
- height-width-inline-table-001d.xht
- height-width-inline-table-001e.xht
- height-width-table-001.xht
- height-width-table-001a.xht
- height-width-table-001b.xht
- height-width-table-001c.xht
- height-width-table-001d.xht
- height-width-table-001e.xht
- padding-applies-to-013a.xht
- row-visibility-001.xht
- row-visibility-002.xht
- separated-border-model-003a.xht
- separated-border-model-003b.xht
- separated-border-model-004a.xht
- separated-border-model-004b.xht
- separated-border-model-004c.xht
- separated-border-model-004d.xht
- separated-border-model-004e.xht
- separated-border-model-007.xht
- separated-border-model-008.xht
- separated-border-model-009.xht
- table-anonymous-objects-017.xht
- table-anonymous-objects-018.xht
- table-anonymous-objects-059.xht
- table-anonymous-objects-060.xht
- table-anonymous-objects-061.xht
- table-anonymous-objects-062.xht
- table-anonymous-objects-063.xht
- table-anonymous-objects-064.xht
- table-anonymous-objects-065.xht
- table-anonymous-objects-066.xht
- table-anonymous-objects-067.xht
- table-anonymous-objects-068.xht
- table-anonymous-objects-069.xht
- table-anonymous-objects-070.xht
- table-anonymous-objects-071.xht
- table-anonymous-objects-072.xht
- table-anonymous-objects-073.xht
- table-anonymous-objects-074.xht
- table-anonymous-objects-075.xht
- table-anonymous-objects-076.xht
- table-anonymous-objects-077.xht
- table-anonymous-objects-078.xht
- table-anonymous-objects-087.xht
- table-anonymous-objects-088.xht
- table-anonymous-objects-089.xht
- table-anonymous-objects-090.xht
- table-anonymous-objects-091.xht
- table-anonymous-objects-092.xht
- table-anonymous-objects-093.xht
- table-anonymous-objects-094.xht
- table-anonymous-objects-095.xht
- table-anonymous-objects-096.xht
- table-anonymous-objects-097.xht
- table-anonymous-objects-098.xht
- table-anonymous-objects-167.xht
- table-anonymous-objects-168.xht
- table-anonymous-objects-177.xht
- table-anonymous-objects-178.xht
- table-anonymous-objects-179.xht
- table-anonymous-objects-180.xht
- table-anonymous-objects-181.xht
- table-anonymous-objects-182.xht
- table-anonymous-objects-183.xht
- table-anonymous-objects-184.xht
- table-anonymous-objects-189.xht
- table-anonymous-objects-190.xht
- table-anonymous-objects-191.xht
- table-anonymous-objects-192.xht
- table-anonymous-objects-197.xht
- table-anonymous-objects-198.xht
- table-anonymous-objects-199.xht
- table-anonymous-objects-200.xht
- table-anonymous-objects-212.xht
- table-anonymous-objects-213.xht
- table-backgrounds-bc-cell-001.xht
- table-backgrounds-bc-colgroup-001.xht
- table-backgrounds-bc-column-001.xht
- table-backgrounds-bc-row-001.xht
- table-backgrounds-bc-rowgroup-001.xht
- table-backgrounds-bc-table-001.xht
- table-backgrounds-bs-cell-001.xht
- table-backgrounds-bs-table-001.xht
- table-cell-001.xht
- table-column-rendering-001.xht
- table-column-rendering-002.xht
- table-footer-group-001.xht
- table-header-group-001.xht
- table-height-algorithm-008a.xht
- table-height-algorithm-008b.xht
- table-height-algorithm-008c.xht
- table-layout-applies-to-016.xht
- table-margin-004.xht
- table-row-group-001.xht
- table-vertical-align-baseline-001.xht
- table-vertical-align-baseline-002.xht
- table-vertical-align-baseline-003.xht
- table-vertical-align-baseline-004.xht
- table-vertical-align-baseline-005.xht
- table-vertical-align-baseline-006.xht
- table-vertical-align-baseline-007.xht
- table-vertical-align-baseline-008.xht
- table-visual-layout-017.xht
- table-visual-layout-018.xht
- table-visual-layout-026a.xht
- table-visual-layout-026b.xht
- table-visual-layout-026c.xht
- table-visual-layout-026d.xht
