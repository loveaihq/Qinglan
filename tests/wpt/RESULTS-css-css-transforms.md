# WPT `css/css-transforms`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/css-transforms`，用时 1.1 秒。

- 通过 **133 / 146**（91.1%）
- bug 类失败：**4**
- 跳过：no reftest 2

失败分类：bug (open) 4，limit (hover) 1，limit (resampled layers) 3，limit (table parts) 5

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| css-transform-scale-001-manual.html | limit (hover) | a manual test: the transform applies on :hover, which needs a pointer (30000 pixels differ (largest difference 255); first at (108,64): #ffffff, the reference has #008000) |
| transform-compound-001.html | limit (resampled layers) | translate, scale, rotate and skew in one transform against the same transforms on nested elements: one resampling against several, edge pixels differ by up to 92 (403 pixels differ (largest difference 92); first at (265,19): #fff4b6, the reference has #fff3b5) |
| transform-display-002.html | bug (open) | display: table with rotate(180deg) against a block; the text of the table is not where the block's text is; not investigated in 2c-2 (a table with the same transform and one short line is right, tests/effects.rs) (4288 pixels differ (largest difference 255); first at (44,28): #ffffff, the reference has #000000) |
| transform-display-003.html | bug (open) | same as transform-display-002 (inline-table) (4288 pixels differ (largest difference 255); first at (44,28): #ffffff, the reference has #000000) |
| transform-generated-001.html | bug (open) | a transform on a floated ::before; not investigated in 2c-2 (renders like css/css-transforms/transform-generated-001-notref.html, which it must not) |
| transform-transformed-caption-contains-fixed-position.html | limit (table parts) | a transform on a table caption, row group or row is painted but is not the containing block of fixed boxes inside it (only blocks, flex and grid containers and tables are) (1974 pixels differ (largest difference 255); first at (33,60): #ffffff, the reference has #a3a3a3) |
| transform-transformed-tbody-contains-fixed-position.html | limit (table parts) | same as the caption test (2691 pixels differ (largest difference 255); first at (65,31): #a2a2a2, the reference has #ffffff) |
| transform-transformed-tfoot-contains-fixed-position.html | limit (table parts) | same as the caption test (2691 pixels differ (largest difference 255); first at (65,31): #a2a2a2, the reference has #ffffff) |
| transform-transformed-thead-contains-fixed-position.html | limit (table parts) | same as the caption test (2691 pixels differ (largest difference 255); first at (65,31): #a2a2a2, the reference has #ffffff) |
| transform-transformed-tr-contains-fixed-position.html | limit (table parts) | same as the caption test (2691 pixels differ (largest difference 255); first at (65,31): #a2a2a2, the reference has #ffffff) |
| transform-transformed-tr-percent-height-child.html | bug (open) | a transformed table row with a percentage-height child; not investigated in 2c-2 (35000 pixels differ (largest difference 255); first at (48,28): #0000ff, the reference has #d3d3d3) |
| transforms-skewX.html | limit (resampled layers) | a skewed or rotated group is drawn upright into a layer and resampled bilinearly, so its edge pixels are blurrier than the exact coverage of the reference's border triangles (the corner pixel is 84% instead of 100%); translate, scale and quarter turns are exact (10199 pixels differ (largest difference 255); first at (50,50): #289428, the reference has #008000) |
| transforms-skewY.html | limit (resampled layers) | same as transforms-skewX (10201 pixels differ (largest difference 255); first at (99,49): #cde6cd, the reference has #ffffff) |

## 通过

- css-scale-nested-001.html
- css-transform-inherit-scale.html
- css-transform-scale-002.html
- css3-transform-scale-002.html
- css3-transform-scale.html
- scale-optional-second-001.html
- scale-zero-001.html
- scalex.html
- scaley.html
- transform-abspos-001.html
- transform-abspos-002.html
- transform-abspos-003.html
- transform-abspos-004.html
- transform-abspos-005.html
- transform-abspos-006.html
- transform-abspos-007.html
- transform-applies-to-001.xht
- transform-applies-to-002.xht
- transform-clip-001.html
- transform-descendant-001.html
- transform-display-001.html
- transform-display-004.html
- transform-generated-002.html
- transform-inherit-001.html
- transform-inherit-002.html
- transform-inherit-origin-001.html
- transform-inherit-origin-002.html
- transform-inline-001.html
- transform-matrix-001.html
- transform-matrix-002.html
- transform-matrix-003.html
- transform-matrix-004.html
- transform-matrix-005.html
- transform-matrix-006.html
- transform-matrix-007.html
- transform-matrix-008.html
- transform-matrix-009.html
- transform-origin-001.html
- transform-origin-002.html
- transform-origin-003.html
- transform-origin-004.html
- transform-origin-005.html
- transform-origin-006.html
- transform-origin-007.html
- transform-origin-008.html
- transform-origin-009.html
- transform-origin-01.html
- transform-origin-010.html
- transform-origin-011.html
- transform-origin-012.html
- transform-origin-013.html
- transform-origin-name-001.html
- transform-origin-name-002.html
- transform-origin-name-003.html
- transform-origin-name-004.html
- transform-origin-name-005.html
- transform-origin-name-006.html
- transform-origin-name-007.html
- transform-origin.html
- transform-percent-001.html
- transform-percent-002.html
- transform-percent-003.html
- transform-percent-004.html
- transform-percent-005.html
- transform-percent-006.html
- transform-percent-007.html
- transform-percent-008.html
- transform-propagate-inherit-boolean-001.html
- transform-rotate-001.html
- transform-rotate-002.html
- transform-rotate-003.html
- transform-rotate-004.html
- transform-rotate-005.html
- transform-rotate-006.html
- transform-rotate-007.html
- transform-rounding-001.html
- transform-scale-001.html
- transform-scale-002.html
- transform-scale-percent-001.html
- transform-scale-test.html
- transform-scalex-001.html
- transform-scaley-001.html
- transform-singular-001.html
- transform-stacking-001.html
- transform-stacking-002.html
- transform-stacking-003.html
- transform-stacking-004.html
- transform-stresstest-001.html
- transform-table-001.html
- transform-table-002.html
- transform-table-003.html
- transform-table-004.html
- transform-table-005.html
- transform-transformable-inline-block.html
- transform-transformable-inline-table.html
- transform-transformable-list-item.html
- transform-transformable-table-caption.html
- transform-transformable-table-cell.html
- transform-transformable-table-footer-group.html
- transform-transformable-table-header-group.html
- transform-transformable-table-row-group.html
- transform-transformable-table-row.html
- transform-transformable-table.html
- transform-transformed-td-contains-fixed-position.html
- transform-transformed-th-contains-fixed-position.html
- transform-translate-001.html
- transform-translate-002.html
- transform-translate-003.html
- transform-translate-004.html
- transform-translate-005.html
- transform-translate-background-001.html
- transform-translatex-001.html
- transform-translatex-002.html
- transform-translatex-003.html
- transform-translatex-004.html
- transform-translatex-005.html
- transform-translatex-006.html
- transform-translatey-001.html
- transform-translatey-002.html
- transform-translatey-003.html
- transform-translatey-004.html
- transform-translatey-005.html
- transform3d-image-scale-001.html
- transform3d-scale-001.html
- transform3d-scale-004.html
- transforms-rotate-degree-90.html
- transforms-rotate-translate-scale.html
- translate-optional-second-001.html
- translate.html
- ttwf-reftest-rotate.html
- ttwf-transform-translatex-001.html
- ttwf-transform-translatey-001.html
- z-index-does-not-apply.html
