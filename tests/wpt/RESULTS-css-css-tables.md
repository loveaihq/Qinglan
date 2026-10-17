# WPT `css/css-tables`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/css-tables`，用时 0.7 秒。

- 通过 **90 / 140**（64.3%）
- bug 类失败：**31**
- 跳过：无

失败分类：bug (open) 31，limit (print) 2，step 2c 7，step 2d 6，step 2e 4

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| absolute-tables-008.tentative.html | bug (open) | absolutely positioned tables with height / insets: not analyzed (10000 pixels differ (largest difference 255); first at (108,48): #008000, the reference has #ffffff) |
| absolute-tables-009.tentative.html | bug (open) | absolutely positioned tables with height / insets: not analyzed (10000 pixels differ (largest difference 255); first at (108,48): #008000, the reference has #ffffff) |
| absolute-tables-010.tentative.html | bug (open) | absolutely positioned tables with height / insets: not analyzed (10000 pixels differ (largest difference 255); first at (108,48): #008000, the reference has #ffffff) |
| absolute-tables-011.tentative.html | bug (open) | absolutely positioned tables with height / insets: not analyzed (800 pixels differ (largest difference 255); first at (0,48): #008000, the reference has #ffffff) |
| baseline-empty-cell-001.html | bug (open) | baseline details of table cells and inline-tables (empty cells, absolutely positioned boxes in a cell); not analyzed (303 pixels differ (largest difference 255); first at (95,8): #ffffff, the reference has #000000) |
| baseline-vertical.html | step 2d | uses text features (37186 pixels differ (largest difference 255); first at (98,8): #000000, the reference has #ffffff) |
| border-collapse-double-border.html | bug (open) | collapsed borders: col / colgroup borders are not part of the conflict resolution, positioned rows / cells, spanning cells with different edge widths, double style (2c); not analyzed one by one (renders like css/css-tables/border-collapse-double-border-notref.html, which it must not) |
| box-shadow-001.html | step 2c | uses painting effects (3600 pixels differ (largest difference 255); first at (28,84): #ff0000, the reference has #008000) |
| calc-percent-plus-0px-auto.html | bug (open) | calc() with a percentage in the width of a table or cell is treated as auto; not analyzed further (976 pixels differ (largest difference 255); first at (140,8): #ffffff, the reference has #000000) |
| calc-percent-plus-0px-fixed.html | bug (open) | calc() with a percentage in the width of a table or cell is treated as auto; not analyzed further (1152 pixels differ (largest difference 255); first at (74,11): #ffffff, the reference has #000000) |
| caption-relative-positioning.html | bug (open) | not analyzed (10000 pixels differ (largest difference 255); first at (8,114): #ff0000, the reference has #008000) |
| col-definite-max-size-001.html | bug (open) | min-width / max-width / width on a col element are not applied (only a fixed width or % width is) (5648 pixels differ (largest difference 255); first at (80,8): #000000, the reference has #ffffff) |
| col-definite-min-size-001.html | bug (open) | min-width / max-width / width on a col element are not applied (only a fixed width or % width is) (2096 pixels differ (largest difference 255); first at (100,8): #ffffff, the reference has #000000) |
| col-definite-size-001.html | bug (open) | min-width / max-width / width on a col element are not applied (only a fixed width or % width is) (360 pixels differ (largest difference 255); first at (80,50): #000000, the reference has #ffffff) |
| collapsed-border-fragmentation-001-print.html | limit (print) | a reftest for paged media (print fragmentation); the harness renders the screen (1440 pixels differ (largest difference 255); first at (100,168): #000000, the reference has #ffffff) |
| collapsed-border-fragmentation-002-print.html | limit (print) | a reftest for paged media (print fragmentation); the harness renders the screen (14768 pixels differ (largest difference 255); first at (0,3): #000000, the reference has #ffffff) |
| collapsed-border-positioned-tr-td.html | bug (open) | collapsed borders: col / colgroup borders are not part of the conflict resolution, positioned rows / cells, spanning cells with different edge widths, double style (2c); not analyzed one by one (613 pixels differ (largest difference 255); first at (8,8): #ffff00, the reference has #0000ff) |
| height-distribution/percentage-sizing-of-table-cell-children-002.html | step 2e | uses svg, scrolling, sticky or form controls (6901 pixels differ (largest difference 255); first at (36,86): #00ffff, the reference has #ffff00) |
| height-distribution/percentage-sizing-of-table-cell-replaced-children-001.html | step 2e | uses svg, scrolling, sticky or form controls (6901 pixels differ (largest difference 255); first at (36,86): #00ffff, the reference has #ffff00) |
| height-distribution/td-different-subpixel-padding-in-same-row-vertical-rl.html | step 2d | uses text features (116 pixels differ (largest difference 255); first at (239,30): #ffffff, the reference has #000000) |
| paint/col-paint-vrl-rtl.html | step 2c | uses painting effects (1408 pixels differ (largest difference 255); first at (41,8): #ffffff, the reference has #000000) |
| percent-height-replaced-in-percent-cell-002.html | bug (open) | a percentage height inside a table cell is treated as auto (the cell height is not definite when its content is laid out) (31735 pixels differ (largest difference 255); first at (8,48): #767676, the reference has #008000) |
| percent-height-replaced-in-percent-cell-003.html | bug (open) | a percentage height inside a table cell is treated as auto (the cell height is not definite when its content is laid out) (9999 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| percent-height-replaced-in-percent-cell-004.html | bug (open) | a percentage height inside a table cell is treated as auto (the cell height is not definite when its content is laid out) (9999 pixels differ (largest difference 255); first at (8,48): #ffffff, the reference has #008000) |
| percentages-grandchildren-quirks-mode-002.html | bug (open) | a percentage height inside a table cell is treated as auto (the cell height is not definite when its content is laid out) (10000 pixels differ (largest difference 255); first at (8,48): #ff0000, the reference has #008000) |
| row-group-margin-border-padding.html | step 2d | uses text features (462 pixels differ (largest difference 255); first at (13,1): #000000, the reference has #ffffff) |
| row-margin-border-padding.html | step 2d | uses text features (462 pixels differ (largest difference 255); first at (13,1): #000000, the reference has #ffffff) |
| rules-groups.html | bug (open) | the HTML rules / frame attributes are not implemented (only the UA colour rule is there) (39639 pixels differ (largest difference 255); first at (25,49): #ffffff, the reference has #000000) |
| table-cell-baseline-static-position.html | bug (open) | baseline details of table cells and inline-tables (empty cells, absolutely positioned boxes in a cell); not analyzed (10000 pixels differ (largest difference 255); first at (58,48): #ffffff, the reference has #008000) |
| table-cell-child-overflow-measure.html | step 2e | uses svg, scrolling, sticky or form controls (10000 pixels differ (largest difference 255); first at (8,108): #008000, the reference has #ffffff) |
| table-cell-overflow-auto.html | step 2e | uses svg, scrolling, sticky or form controls (1000 pixels differ (largest difference 255); first at (111,61): #008000, the reference has #000000) |
| table-intrinsic-size-001.html | bug (open) | the min-content size of a table whose width is smaller than its content (shrink-to-fit outer box): not analyzed (2000 pixels differ (largest difference 255); first at (78,48): #ffffff, the reference has #008000) |
| table-intrinsic-size-002.html | bug (open) | the min-content size of a table whose width is smaller than its content (shrink-to-fit outer box): not analyzed (2000 pixels differ (largest difference 255); first at (78,48): #ffffff, the reference has #008000) |
| table-intrinsic-size-003.html | step 2d | uses text features (4000 pixels differ (largest difference 255); first at (58,48): #ffffff, the reference has #008000) |
| table-intrinsic-size-004.html | step 2d | uses text features (4000 pixels differ (largest difference 255); first at (58,48): #ffffff, the reference has #008000) |
| tentative/border-collapse-spanning-cells-002.html | bug (open) | collapsed borders: col / colgroup borders are not part of the conflict resolution, positioned rows / cells, spanning cells with different edge widths, double style (2c); not analyzed one by one (2400 pixels differ (largest difference 255); first at (78,114): #ff0000, the reference has #008000) |
| tentative/border-collapse-spanning-cells-003.html | bug (open) | collapsed borders: col / colgroup borders are not part of the conflict resolution, positioned rows / cells, spanning cells with different edge widths, double style (2c); not analyzed one by one (1200 pixels differ (largest difference 255); first at (78,154): #ff0000, the reference has #008000) |
| tentative/border-collapse-spanning-cells-004.html | bug (open) | collapsed borders: col / colgroup borders are not part of the conflict resolution, positioned rows / cells, spanning cells with different edge widths, double style (2c); not analyzed one by one (3200 pixels differ (largest difference 255); first at (78,114): #ff0000, the reference has #008000) |
| tentative/padding-percentage.html | bug (open) | percentage padding of cells: not analyzed (3000 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| tentative/paint/background-image-column-collapsed.html | step 2c | uses painting effects (5000 pixels differ (largest difference 111); first at (113,63): #ffffff, the reference has #90ee90) |
| tentative/paint/background-image-column.html | step 2c | uses painting effects (8712 pixels differ (largest difference 111); first at (38,28): #90ee90, the reference has #ffffff) |
| tentative/paint/background-image-row-collapsed.html | step 2c | uses painting effects (5000 pixels differ (largest difference 111); first at (55,105): #ffffff, the reference has #90ee90) |
| tentative/paint/background-image-row.html | step 2c | uses painting effects (9100 pixels differ (largest difference 111); first at (88,98): #90ee90, the reference has #ffffff) |
| tentative/paint/collapsed-border-large-cell.html | bug (open) | collapsed borders: col / colgroup borders are not part of the conflict resolution, positioned rows / cells, spanning cells with different edge widths, double style (2c); not analyzed one by one (6971 pixels differ (largest difference 255); first at (8,8): #000000, the reference has #0000ff) |
| tentative/paint/overflow-hidden-table.html | bug (open) | not analyzed (6712 pixels differ (largest difference 255); first at (98,8): #008000, the reference has #ffffff) |
| th-text-align.html | bug (open) | th uses text-align: center; browsers use -internal-center (the parent value wins when it is set); not done (450 pixels differ (largest difference 255); first at (80,64): #000000, the reference has #ffffff) |
| visibility-collapse-border-spacing-001.html | step 2c | uses painting effects (3600 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| visibility-collapse-border-spacing-002.html | bug (open) | visibility: collapse is done for rows (the row and its spacing go); columns, and cells that span collapsed rows or columns, are not (10000 pixels differ (largest difference 255); first at (58,164): #ff0000, the reference has #008000) |
| visibility-collapse-colspan-003.html | bug (open) | visibility: collapse is done for rows (the row and its spacing go); columns, and cells that span collapsed rows or columns, are not (67695 pixels differ (largest difference 255); first at (77,8): #ff0000, the reference has #ffffff) |
| visibility-collapse-rowspan-005.html | bug (open) | visibility: collapse is done for rows (the row and its spacing go); columns, and cells that span collapsed rows or columns, are not (4176 pixels differ (largest difference 255); first at (380,145): #000000, the reference has #ffffff) |

## 通过

- absolute-tables-007.html
- absolute-tables-012.html
- absolute-tables-013.html
- absolute-tables-014.html
- absolute-tables-015.html
- absolute-tables-016.html
- anonymous-table-cell-margin-collapsing.html
- anonymous-table-ws-001.html
- background-clip-001.html
- border-collapse-empty-cell.html
- border-collapse-rowspan-cell.html
- border-conflict-resolution.html
- collapsed-border-large-rowgroup-001.html
- collapsed-border-large-rowgroup-002.html
- collapsed-border-paint-phase-001.html
- collapsed-border-paint-phase-002.html
- collapsed-border-sideways-rl-rtl-overflow.html
- collapsed-border-vertical-lr-rtl-overflow.html
- collapsed-border-vertical-rtl-overflow.html
- colspan-004.html
- display-contents-001.html
- display-contents-002.html
- display-contents-003.html
- empty-table-height.html
- height-distribution/extra-height-given-to-all-row-groups-001.html
- height-distribution/extra-height-given-to-all-row-groups-002.html
- height-distribution/extra-height-given-to-all-row-groups-005.html
- height-distribution/percentage-sizing-of-table-cell-007.html
- height-distribution/percentage-sizing-of-table-cell-children-003.html
- height-distribution/percentage-sizing-of-table-cell-children-004.html
- height-distribution/percentage-sizing-of-table-cell-children-005.html
- height-distribution/percentage-sizing-of-table-cell-children-006.html
- height-distribution/td-different-subpixel-padding-in-same-row.html
- html-display-table.html
- internal-containing-block-001.html
- max-height-table.html
- min-height-table-2.html
- min-height-table.html
- min-max-size-table-content-box.html
- out-of-order-elements-collapsed-border.html
- paint/col-paint-htb-rtl.html
- percent-height-overflow-auto-in-unrestricted-block-size-cell.tentative.html
- percent-height-replaced-in-percent-cell.tentative.html
- percent-height-table-cell-child.html
- percentages-grandchildren-quirks-mode-001.html
- row-group-order.html
- rowspan-cell-border-after-color.html
- subpixel-collapsed-borders-001.html
- subpixel-collapsed-borders-002.html
- subpixel-collapsed-borders-003.html
- subpixel-table-cell-height-001.html
- subpixel-table-cell-width-001.html
- subpixel-table-cell-width-002.html
- subpixel-table-width-001.html
- table-as-item-cell-percentage-001.html
- table-as-item-cell-percentage-002.html
- table-as-item-cell-percentage-003.html
- table-as-item-cell-percentage-004.html
- table-cell-inline-size-box-sizing-quirks.html
- table-cell-overflow-explicit-height-001.html
- table-cell-overflow-explicit-height-002.html
- table-cell-tiny-width-excess-width-distribution.html
- table-cell-width-0-excess-width-distribution.html
- table-cell-width-0.html
- table-colspan-percent-auto.html
- table-has-box-sizing-border-box-001.html
- table-has-box-sizing-border-box-002.html
- table-row-group-color-inheritance-001.html
- table-row-group-nested-anonymous-001.html
- table_grid_size_col_colspan.html
- tentative/border-collapse-spanning-cells-001.html
- tentative/collapsed-borders-painting-order-001.html
- tentative/collapsed-borders-painting-order-002.html
- tentative/collapsed-borders-painting-order-003.html
- tentative/collapsed-borders-painting-order-004.html
- tentative/collapsed-borders-painting-order-005.html
- tentative/collapsed-borders-painting-order-006.html
- tentative/collapsed-borders-painting-order-007.html
- tentative/collapsed-borders-painting-order-008.html
- tentative/collapsed-borders-painting-order-009.html
- tentative/collapsed-borders-painting-order-010.html
- tentative/collapsed-borders-painting-order-011.html
- tentative/collapsed-borders-painting-order-012.html
- tentative/collapsed-borders-painting-order-013.html
- tentative/section-no-tbody-fixed-distribution.html
- tentative/section-no-tbody-percent-distribution.html
- visibility-hidden-collapsed-borders.html
- whitespace-001.html
- zero-rowspan-001.html
- zero-rowspan-002.html
