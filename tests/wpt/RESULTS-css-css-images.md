# WPT `css/css-images`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/css-images`，用时 1.2 秒。

- 通过 **59 / 129**（45.7%）
- bug 类失败：**0**
- 跳过：无

失败分类：limit (calc in gradients) 5，limit (gradient color spaces) 18，limit (image()) 2，limit (relative colors, ::first-line) 1，not planned 44

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| conic-gradient-calc-angle-percentage.html | limit (calc in gradients) | calc() with infinity / calc() as a conic angle or stop position is not done; the declaration is dropped (30000 pixels differ (largest difference 255); first at (8,8): #ff0000, the reference has #00ff00) |
| css-image-fallbacks-and-annotations.html | not planned | the image() function (colors and fallbacks as images) is not done (40000 pixels differ (largest difference 255); first at (8,48): #ff0000, the reference has #008000) |
| css-image-fallbacks-and-annotations002.html | not planned | the image() function (colors and fallbacks as images) is not done (40000 pixels differ (largest difference 255); first at (8,48): #ff0000, the reference has #008000) |
| css-image-fallbacks-and-annotations003.html | limit (image()) | the `image()` notation (with fallbacks) is not implemented (40000 pixels differ (largest difference 255); first at (8,48): #ff0000, the reference has #008000) |
| css-image-fallbacks-and-annotations004.html | limit (image()) | the `image()` notation (with fallbacks) is not implemented (40000 pixels differ (largest difference 255); first at (8,48): #ff0000, the reference has #008000) |
| css-image-fallbacks-and-annotations005.html | not planned | the image() function (colors and fallbacks as images) is not done (48320 pixels differ (largest difference 255); first at (536,16): #ffffff, the reference has #000000) |
| gradient/conic-gradient-001.html | limit (calc in gradients) | calc() with infinity / calc() as a conic angle or stop position is not done; the declaration is dropped (56172 pixels differ (largest difference 255); first at (178,48): #ff0000, the reference has #00ff00) |
| gradient/display-p3-linear-gradient.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (23900 pixels differ (largest difference 74); first at (9,8): #fe0100, the reference has #ff0700) |
| gradient/gradient-decreasing-hue-hsl.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (64400 pixels differ (largest difference 255); first at (18,10): #ff0100, the reference has #fe0004) |
| gradient/gradient-decreasing-hue-lch.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (79700 pixels differ (largest difference 235); first at (18,10): #ff0080, the reference has #fb0182) |
| gradient/gradient-eval-002.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (10000 pixels differ (largest difference 21); first at (8,8): #7c898b, the reference has #919191) |
| gradient/gradient-eval-003.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (10000 pixels differ (largest difference 20); first at (8,8): #7d918d, the reference has #919191) |
| gradient/gradient-eval-004.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (10000 pixels differ (largest difference 28); first at (8,8): #00007f, the reference has #000063) |
| gradient/gradient-eval-005.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (10000 pixels differ (largest difference 72); first at (8,8): #a4848d, the reference has #ec5c94) |
| gradient/gradient-eval-006.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (10000 pixels differ (largest difference 58); first at (8,8): #7fb380, the reference has #b9b9b9) |
| gradient/gradient-eval-007.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (10000 pixels differ (largest difference 73); first at (8,8): #a3858e, the reference has #ec5c94) |
| gradient/gradient-eval-008.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (10000 pixels differ (largest difference 236); first at (8,8): #00aa8f, the reference has #ec5c94) |
| gradient/gradient-eval-009.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (10000 pixels differ (largest difference 73); first at (8,8): #a3858e, the reference has #ec5c94) |
| gradient/gradient-increasing-hue-hsl.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (65300 pixels differ (largest difference 255); first at (19,10): #ff0100, the reference has #ff0200) |
| gradient/gradient-increasing-hue-lch.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (79700 pixels differ (largest difference 235); first at (18,10): #ff0080, the reference has #ff007f) |
| gradient/gradient-infinity-001.html | limit (calc in gradients) | calc() with infinity / calc() as a conic angle or stop position is not done; the declaration is dropped (30000 pixels differ (largest difference 255); first at (121,9): #ffffff, the reference has #00ff00) |
| gradient/gradient-infinity-002.html | limit (calc in gradients) | calc() with infinity / calc() as a conic angle or stop position is not done; the declaration is dropped (31212 pixels differ (largest difference 255); first at (123,9): #ffffff, the reference has #00ff00) |
| gradient/gradient-infinity-003.html | limit (calc in gradients) | calc() with infinity / calc() as a conic angle or stop position is not done; the declaration is dropped (8096 pixels differ (largest difference 255); first at (72,8): #ffffff, the reference has #00ff00) |
| gradient/gradient-single-stop-longer-hue-hsl.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (90000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #ff0000) |
| gradient/gradient-single-stop-longer-hue-oklch.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (90000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #ff0000) |
| gradient/linear-gradient-relative-currentcolor-stop.html | limit (relative colors, ::first-line) | relative color syntax / ::first-line backgrounds are not done (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #0000ff) |
| gradient/oklab-gradient.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (23900 pixels differ (largest difference 97); first at (9,8): #fe0100, the reference has #ff0400) |
| gradient/srgb-linear-gradient.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (23900 pixels differ (largest difference 74); first at (9,8): #fe0100, the reference has #ff0700) |
| gradient/xyz-gradient.html | limit (gradient color spaces) | gradients interpolate in premultiplied sRGB only; `in <color-space>` is parsed and ignored (decisions.md 2c-1) (23900 pixels differ (largest difference 74); first at (9,8): #fe0100, the reference has #ff0700) |
| image-color-background-combined.html | not planned | the image() function (colors and fallbacks as images) is not done (14800 pixels differ (largest difference 255); first at (320,112): #ff0000, the reference has #008000) |
| image-color-background-position.html | not planned | the image() function (colors and fallbacks as images) is not done (10000 pixels differ (largest difference 255); first at (8,80): #ff0000, the reference has #008000) |
| image-color-background-repeat.html | not planned | the image() function (colors and fallbacks as images) is not done (90000 pixels differ (largest difference 255); first at (8,80): #ff0000, the reference has #008000) |
| image-color-background-size.html | not planned | the image() function (colors and fallbacks as images) is not done (42500 pixels differ (largest difference 255); first at (8,80): #ff0000, the reference has #008000) |
| image-color-basic.html | not planned | the image() function (colors and fallbacks as images) is not done (40000 pixels differ (largest difference 255); first at (12,68): #ff0000, the reference has #008000) |
| image-color-layered.html | not planned | the image() function (colors and fallbacks as images) is not done (20000 pixels differ (largest difference 255); first at (12,52): #ff0000, the reference has #008000) |
| image-orientation/image-orientation-background-image.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (5029 pixels differ (largest difference 255); first at (8,8): #008001, the reference has #ffff00) |
| image-orientation/image-orientation-background-position.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (3301 pixels differ (largest difference 255); first at (10,10): #008001, the reference has #ffff00) |
| image-orientation/image-orientation-background-properties-border-radius.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (82933 pixels differ (largest difference 255); first at (45,20): #008001, the reference has #0000fe) |
| image-orientation/image-orientation-background-properties.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (82953 pixels differ (largest difference 255); first at (45,20): #008001, the reference has #0000fe) |
| image-orientation/image-orientation-default.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (95069 pixels differ (largest difference 255); first at (124,80): #008001, the reference has #ffff00) |
| image-orientation/image-orientation-exif-png-2.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (22427 pixels differ (largest difference 255); first at (129,56): #fcfcfc, the reference has #ffffff) |
| image-orientation/image-orientation-exif-png-3.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (22427 pixels differ (largest difference 255); first at (129,56): #fcfcfc, the reference has #ffffff) |
| image-orientation/image-orientation-exif-png.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (22427 pixels differ (largest difference 255); first at (129,56): #fcfcfc, the reference has #ffffff) |
| image-orientation/image-orientation-from-image-composited.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (95069 pixels differ (largest difference 255); first at (124,80): #008001, the reference has #ffff00) |
| image-orientation/image-orientation-from-image-content-images.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (27521 pixels differ (largest difference 255); first at (33,240): #008001, the reference has #0000fe) |
| image-orientation/image-orientation-from-image.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (95069 pixels differ (largest difference 255); first at (124,80): #008001, the reference has #ffff00) |
| image-orientation/image-orientation-img-object-fit.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (10222 pixels differ (largest difference 255); first at (59,10): #008001, the reference has #004080) |
| image-rendering-mixed-scaled.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (renders like css/css-images/image-rendering-mixed-scaled-notref.html, which it must not) |
| object-fit-contain-png-001i.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (13528 pixels differ (largest difference 255); first at (62,10): #0000ff, the reference has #ffffff) |
| object-fit-contain-png-002i.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (13753 pixels differ (largest difference 255); first at (10,10): #0000ff, the reference has #ffffff) |
| object-fit-containcontainintrinsicsize-png-001i.tentative.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (5021 pixels differ (largest difference 255); first at (13,8): #bfbfbf, the reference has #c7c7c7) |
| object-fit-containsize-png-001i.tentative.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (5296 pixels differ (largest difference 255); first at (25,10): #000080, the reference has #0000ff) |
| object-fit-cover-png-001i.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (7433 pixels differ (largest difference 255); first at (23,10): #0000ff, the reference has #0000bf) |
| object-fit-cover-png-002i.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (7208 pixels differ (largest difference 255); first at (62,10): #0000ff, the reference has #ff8080) |
| object-fit-fill-png-001i.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (1703 pixels differ (largest difference 255); first at (186,10): #0000ff, the reference has #0000aa) |
| object-fit-fill-png-002i.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (1521 pixels differ (largest difference 255); first at (183,10): #0000ff, the reference has #0000d5) |
| object-fit-none-png-001i.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (21094 pixels differ (largest difference 255); first at (10,10): #0000ff, the reference has #ffffff) |
| object-fit-none-png-002i.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (21090 pixels differ (largest difference 255); first at (10,10): #0000ff, the reference has #ffffff) |
| object-fit-scale-down-png-001i.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (21190 pixels differ (largest difference 255); first at (10,10): #0000ff, the reference has #ffffff) |
| object-fit-scale-down-png-002i.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (21206 pixels differ (largest difference 255); first at (10,10): #0000ff, the reference has #ffffff) |
| object-position-png-001i.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (2080 pixels differ (largest difference 255); first at (30,8): #0000ff, the reference has #d3d3d3) |
| object-position-png-002i.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (2080 pixels differ (largest difference 255); first at (11,8): #0000ff, the reference has #0000cc) |
| object-view-box-rect-percentage.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (5000 pixels differ (largest difference 255); first at (8,8): #008001, the reference has #ffffff) |
| object-view-box-rect.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (5000 pixels differ (largest difference 255); first at (8,8): #008001, the reference has #ffffff) |
| object-view-box-xywh-percentage.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (5000 pixels differ (largest difference 255); first at (8,8): #008001, the reference has #ffffff) |
| object-view-box-xywh.html | not planned | object-fit, object-position, object-view-box, image-orientation and image-rendering of replaced elements are not in the layer-2 list (5000 pixels differ (largest difference 255); first at (8,8): #008001, the reference has #ffffff) |
| radial-gradient-container-relative-units-001.html | not planned | container query units (cqw ...) are not done (no @container) (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #0000ff) |
| radial-gradient-container-relative-units-002.html | not planned | container query units (cqw ...) are not done (no @container) (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #0000ff) |
| radial-gradient-container-relative-units-003.html | not planned | container query units (cqw ...) are not done (no @container) (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #0000ff) |
| radial-gradient-container-relative-units-004.html | not planned | container query units (cqw ...) are not done (no @container) (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #0000ff) |

## 通过

- color-stop-currentcolor.html
- conic-gradient-angle-negative.html
- conic-gradient-angle.html
- conic-gradient-center.html
- gradient/gradient-analogous-missing-components-001.html
- gradient/gradient-analogous-missing-components-004.html
- gradient/gradient-eval-001.html
- gradient/gradient-hue-direction.html
- gradient/gradient-longer-hue-hsl-013.html
- gradient/gradient-single-stop-001.html
- gradient/gradient-single-stop-002.html
- gradient/gradient-single-stop-003.html
- gradient/gradient-single-stop-004.html
- gradient/gradient-single-stop-005.html
- gradient/gradient-single-stop-006.html
- gradient/gradient-single-stop-007.html
- gradient/gradient-single-stop-008.html
- gradient/gradient-single-stop-longer-hue-hsl-002.html
- gradient/gradient-to-transparent.html
- gradient/legacy-color-gradient.html
- gradient/repeating-gradient-hsl-and-oklch.html
- gradient/srgb-gradient.html
- gradient-border-box.html
- gradient-button.html
- gradient-content-box.html
- gradient-move-stops.html
- gradient-refcrash.html
- gradients-with-border.html
- gradients-with-transparent.html
- image-orientation/image-orientation-list-style-image.html
- image-orientation/image-orientation-none-content-images.html
- image-orientation/image-orientation-none.html
- infinite-radial-gradient-refcrash.html
- linear-gradient-1.html
- linear-gradient-2.html
- linear-gradient-calc-em-units.html
- linear-gradient-non-square.html
- multiple-position-color-stop-conic-2.html
- normalization-conic-2.html
- normalization-conic-degenerate.html
- normalization-conic.html
- normalization-linear-2.html
- normalization-linear-degenerate.html
- normalization-linear.html
- normalization-radial-2.html
- normalization-radial-3.html
- normalization-radial-4.html
- normalization-radial-degenerate.html
- normalization-radial.html
- object-view-box-empty-bounds.html
- object-view-box-negative-bounds.html
- object-view-box-rect-auto.html
- object-view-box-same-size.html
- out-of-range-color-stop-conic.html
- repeating-conic-gradient.html
- svg-images-are-ignored.html
- tiled-conic-gradients.html
- tiled-gradients.html
- tiled-radial-gradients.html
