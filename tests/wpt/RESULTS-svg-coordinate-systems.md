# WPT `svg/coordinate-systems`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt svg/coordinate-systems`，用时 0.1 秒。

- 通过 **9 / 13**（69.2%）
- bug 类失败：**0**
- 跳过：无

失败分类：limit (abspos replaced) 2，limit (svg view) 1，limit (zero width) 1

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| abspos.html | limit (abspos replaced) | an absolutely positioned `svg` with an aspect ratio but no size takes its width from left/right (CSS2 10.3.8 with a ratio): the engine uses the 300px default (45370 pixels differ (largest difference 255); first at (104,18): #ffffff, the reference has #eff7ef) |
| out-of-flow-svg-root.html | limit (abspos replaced) | an absolutely positioned `svg` with an aspect ratio but no size takes its width from left/right (CSS2 10.3.8 with a ratio): the engine uses the 300px default (32400 pixels differ (largest difference 38); first at (248,8): #d9d9ff, the reference has #ffffff) |
| outer-svg-intrinsic-size-003.html | limit (zero width) | an SVG image with `width=0` and a viewBox: the zero intrinsic width is not turned into "no intrinsic width" with a ratio (43600 pixels differ (largest difference 255); first at (8,164): #008000, the reference has #ffffff) |
| view-transform-viewBox.html | limit (svg view) | SVG view fragments (`file.svg#transform` naming a `<view>`) are not implemented (1664 pixels differ (largest difference 255); first at (0,0): #008000, the reference has #ffffff) |

## 通过

- outer-svg-intrinsic-size-004.html
- outer-svg-intrinsic-size-005.html
- view-invalid-viewBox.html
- viewBox-fractional-valid.html
- viewBox-zero-disables-rendering-001.svg
- viewBox-zero-disables-rendering-002.svg
- viewBox-zero-disables-rendering-003.svg
- viewBox-zero-disables-rendering-contexts.html
- viewBox-zero-disables-rendering-symbol.svg
