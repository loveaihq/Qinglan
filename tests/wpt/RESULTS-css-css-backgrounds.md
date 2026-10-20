# WPT `css/css-backgrounds`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/css-backgrounds`，用时 1.8 秒。

- 通过 **220 / 237**（92.8%）
- bug 类失败：**6**
- 跳过：无

失败分类：bug (open) 6，limit (background-attachment) 5，limit (blur) 1，limit (contain) 2，limit (gradient color spaces) 1，limit (relative colors, ::first-line) 1，limit (spec change) 1

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| background-attachment-350.html | limit (background-attachment) | background-attachment fixed and local are painted as scroll (task 2c: recorded in decisions.md) (10000 pixels differ (largest difference 255); first at (8,48): #ff0000, the reference has #ffffff) |
| background-attachment-local-hidden.html | limit (background-attachment) | background-attachment fixed and local are painted as scroll (task 2c: recorded in decisions.md) (4056 pixels differ (largest difference 41); first at (39,8): #f8eff1, the reference has #fff3f3) |
| background-attachment-margin-root-002.html | limit (background-attachment) | background-attachment fixed and local are painted as scroll (task 2c: recorded in decisions.md) (480000 pixels differ (largest difference 64); first at (0,0): #003f41, the reference has #007f01) |
| background-color-body-propagation-008.html | limit (contain) | `contain: paint` on the root stops the propagation of the body background; `contain` is not implemented (467456 pixels differ (largest difference 255); first at (0,0): #008000, the reference has #ffffff) |
| background-color-body-propagation-009.html | limit (contain) | `contain: paint` on the root stops the propagation of the body background; `contain` is not implemented (467456 pixels differ (largest difference 255); first at (0,0): #008000, the reference has #ffffff) |
| background-gradient-interpolation-001.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (renders like css/css-backgrounds/background-gradient-interpolation-001-notref.html, which it must not) |
| background-image-centered-with-border-radius.html | bug (open) | 28 pixels differ by up to 63 at the arc where a centered picture meets the rounded edge; not investigated in 2c-1 (picture resampling at the clip edge) (28 pixels differ (largest difference 63); first at (10,8): #afa078, the reference has #78bc78) |
| background-origin/origin-border-box_with_position.html | bug (open) | the reference mixes a div with background-origin: border-box under a semi-transparent border and an <img> with a radius; not investigated in 2c-1 (2384 pixels differ (largest difference 255); first at (0,0): #ffffff, the reference has #ffff00) |
| background-origin/origin-padding-box_with_radius.html | bug (open) | the reference mixes a div with background-origin: border-box under a semi-transparent border and an <img> with a radius; not investigated in 2c-1 (10 pixels differ (largest difference 255); first at (24,24): #ffffff, the reference has #ffff00) |
| background-origin-006.html | limit (background-attachment) | background-attachment fixed and local are painted as scroll (task 2c: recorded in decisions.md) (5400 pixels differ (largest difference 255); first at (0,0): #ff0000, the reference has #008000) |
| border-radius-012.html | bug (open) | an inline box with a radius broken over two lines: the arcs differ by anti-aliasing and 1px; not investigated in 2c-1 (1676 pixels differ (largest difference 255); first at (21,58): #fdfdfd, the reference has #ffffff) |
| border-radius-clip-002.htm | bug (open) | overflow clip with a radius around positioned / transformed-looking content differs at the arcs; not investigated in 2c-1 (67 pixels differ (largest difference 97); first at (39,86): #050404, the reference has #050505) |
| box-shadow-border-radius-001.html | limit (spec change) | csswg-drafts#7103 asks for a circle; css-backgrounds-3 6.1.1 (cited in the code) gives the radius 1 + 149 * (1 + (1/149 - 1)^3) = 4px, which is the notref (renders like css/css-backgrounds/reference/box-shadow-border-radius-001-notref.html, which it must not) |
| box-shadow-overlapping-001.html | bug (open) | the shadow of an inline box (span) is painted with its background, before the text of the line, so it does not cover the red text of the preceding sibling; the inline box fragments come first; not fixed in 2c-2 (30000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| box-shadow-radius-000.html | limit (blur) | three box blurs against the reference's blur: up to 5/255 apart (the spec allows 5%) (104 pixels differ (largest difference 5); first at (238,73): #c2bfbf, the reference has #c2c2c2) |
| linear-gradient-currentcolor-first-line.html | limit (relative colors, ::first-line) | relative color syntax / ::first-line backgrounds are not done (1280 pixels differ (largest difference 255); first at (8,48): #ff0000, the reference has #008000) |
| table-cell-background-local-002.html | limit (background-attachment) | background-attachment fixed and local are painted as scroll (task 2c: recorded in decisions.md) (10000 pixels differ (largest difference 255); first at (112,168): #ffffff, the reference has #008000) |

## 通过

- background-334.html
- background-attachment-margin-root-001.html
- background-clip/clip-rounded-corner.html
- background-clip-002.html
- background-clip-003.html
- background-clip-004.html
- background-clip-005.html
- background-clip-006.html
- background-clip-007.html
- background-clip-008.html
- background-clip-009.html
- background-clip-010.html
- background-clip-color.html
- background-clip-content-box-001.html
- background-clip-content-box-002.html
- background-clip-content-box-with-border-radius-002.html
- background-clip-content-box-with-border-radius-003.html
- background-clip-padding-box-001.html
- background-clip-padding-box-with-border-radius-002.html
- background-clip-padding-box-with-border-radius-003.html
- background-clip-padding-box-with-border-radius.html
- background-clip_padding-box.html
- background-color-body-propagation-001.html
- background-color-body-propagation-002.html
- background-color-body-propagation-004.html
- background-color-body-propagation-005.html
- background-color-body-propagation-007.html
- background-color-body-propagation-010.html
- background-color-clip.html
- background-color-root-propagation-001.html
- background-color-root-propagation-003.html
- background-image-001.html
- background-image-002.html
- background-image-003.html
- background-image-004.html
- background-image-005.html
- background-image-006.html
- background-image-centered.html
- background-image-cover-zoomed-1.html
- background-image-large-with-auto.html
- background-image-table-cells-straddling-no-repeat.html
- background-image-table-cells-zoomed.html
- background-image-with-border-radius-fidelity.html
- background-margin-root.html
- background-margin-will-change-root.html
- background-multiple-layers-table-cell.html
- background-origin/origin-border-box.html
- background-origin/origin-border-box_with_radius.html
- background-origin/origin-border-box_with_size.html
- background-origin/origin-content-box.html
- background-origin/origin-content-box_with_position.html
- background-origin/origin-content-box_with_radius.html
- background-origin/origin-content-box_with_size.html
- background-origin/origin-padding-box.html
- background-origin/origin-padding-box_with_position.html
- background-origin/origin-padding-box_with_size.html
- background-origin-002.html
- background-origin-003.html
- background-origin-004.html
- background-origin-005.html
- background-origin-007.html
- background-origin-008.html
- background-paint-order-001.html
- background-position/background-position-bottom-right-repeat-round.html
- background-position/subpixel-position-center.tentative.html
- background-position-calc-minmax-001.html
- background-position-negative-percentage-comparison-002.html
- background-position-negative-percentage-comparison.html
- background-position-subpixel-at-border.tentative.html
- background-position-three-four-values.html
- background-repeat/background-repeat-no-repeat.xht
- background-repeat/background-repeat-repeat-x.xht
- background-repeat/background-repeat-repeat-y.xht
- background-repeat/background-repeat-round-roundup.xht
- background-repeat/background-repeat-round.xht
- background-repeat/background-repeat-space.xht
- background-repeat/gradient-repeat-spaced-with-borders.html
- background-repeat-round-1a.html
- background-repeat-round-1b.html
- background-repeat-round-1c.html
- background-repeat-round-1d.html
- background-repeat-round-1e.html
- background-repeat-round-2.html
- background-repeat-round-3.html
- background-repeat-round-4.html
- background-repeat-space-10.html
- background-repeat-space-1a.html
- background-repeat-space-1b.html
- background-repeat-space-1c.html
- background-repeat-space-2.html
- background-repeat-space-3.html
- background-repeat-space-4.html
- background-repeat-space-5.html
- background-repeat-space-6.html
- background-repeat-space-7.html
- background-repeat-space-8.html
- background-repeat-space-9.html
- background-rounded-image-clip-001.html
- background-size/background-size-contain.xht
- background-size/background-size-cover-contain-001.xht
- background-size/background-size-cover-contain-002.xht
- background-size/background-size-cover.xht
- background-size/background-size-near-zero-color.html
- background-size/background-size-near-zero-gradient.html
- background-size/background-size-near-zero-png.html
- background-size-002.html
- background-size-005.html
- background-size-006.html
- background-size-007.html
- background-size-008.html
- background-size-009.html
- background-size-010.html
- background-size-011.html
- background-size-012.html
- background-size-013.html
- background-size-014.html
- background-size-015.html
- background-size-016.html
- background-size-017.html
- background-size-018.html
- background-size-019.html
- background-size-020.html
- background-size-021.html
- background-size-025.html
- background-size-026.html
- background-size-027.html
- background-size-028.html
- background-size-029.html
- background-size-030.html
- background-size-031.html
- background-size-034.html
- background-size-041.html
- background-size-042.html
- background-size-contain-001.html
- background-size-contain-002.html
- background-size-cover-001.html
- background-size-cover-002.html
- background-size-cover-003.html
- background-size-one-value-1x1-image.html
- background-size-percentage-root.html
- background-size-with-negative-value.html
- border-bottom-left-radius-004.xht
- border-bottom-left-radius-005.xht
- border-bottom-right-radius-004.xht
- border-bottom-right-radius-005.xht
- border-bottom-width-medium.html
- border-bottom-width-thick.html
- border-bottom-width-thin.html
- border-left-width-medium.html
- border-left-width-thick.html
- border-left-width-thin.html
- border-radius-001.xht
- border-radius-002.xht
- border-radius-003.xht
- border-radius-004.xht
- border-radius-005.xht
- border-radius-006.xht
- border-radius-007.xht
- border-radius-008.xht
- border-radius-009.xht
- border-radius-010.xht
- border-radius-011.xht
- border-radius-013.html
- border-radius-clip-001.html
- border-radius-clipping-002.html
- border-radius-horizontal-value-is-zero.html
- border-radius-shorthand-002.html
- border-right-width-medium.html
- border-right-width-thick.html
- border-right-width-thin.html
- border-top-left-radius-004.xht
- border-top-left-radius-005.xht
- border-top-right-radius-004.xht
- border-top-right-radius-005.xht
- border-top-width-medium.html
- border-top-width-thick.html
- border-top-width-thin.html
- border-width-pixel-snapping-001-a.html
- border-width-pixel-snapping-001-b.html
- border-width-small-values-001-a.html
- border-width-small-values-001-b.html
- border-width-small-values-001-c.html
- border-width-small-values-001-d.html
- border-width-small-values-001-e.html
- box-shadow-005.html
- box-shadow-029.html
- box-shadow-039.html
- box-shadow-040.html
- box-shadow-041.html
- box-shadow-042.html
- box-shadow-body.html
- box-shadow-calc.html
- box-shadow-currentcolor.html
- box-shadow-inset-without-border-radius.html
- box-shadow-invalid-001.html
- box-shadow-multiple-001.html
- box-shadow-outset-without-border-radius-001.html
- box-shadow-overlapping-002.html
- box-shadow-overlapping-003.html
- box-shadow-overlapping-004.html
- box-shadow-radius-001.html
- box-shadow-radius-002.html
- box-shadow-table-border-collapse-001.html
- box-shadow-table-row-display.html
- css-border-radius-001.html
- css3-background-clip-border-box.html
- css3-background-clip.html
- css3-background-origin-border-box.html
- css3-background-origin-content-box.html
- css3-background-origin-padding-box.html
- css3-background-size-001.html
- css3-background-size-contain.html
- css3-background-size.html
- css3-box-shadow.html
- fieldset-inset-shadow.html
- first-letter-space-not-selected.html
- inline-background-rtl-001.html
- order-of-images.htm
- subpixel-repeat-no-repeat-mix.html
- ttwf-reftest-borderRadius.html
