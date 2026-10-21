# WPT `svg/path`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt svg/path`，用时 0.2 秒。

- 通过 **5 / 14**（35.7%）
- bug 类失败：**0**
- 跳过：无

失败分类：limit (CSS d) 2，limit (CSS path-length) 4，limit (SVG 2 bearing) 3

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| bearing/absolute.svg | limit (SVG 2 bearing) | path data with the `B`/`b` bearing commands of an old SVG 2 draft (dropped from the spec): not implemented (17856 pixels differ (largest difference 255); first at (18,28): #ffffff, the reference has #bfbfff) |
| bearing/relative.svg | limit (SVG 2 bearing) | path data with the `B`/`b` bearing commands of an old SVG 2 draft (dropped from the spec): not implemented (17856 pixels differ (largest difference 255); first at (18,28): #ffffff, the reference has #bfbfff) |
| bearing/zero.svg | limit (SVG 2 bearing) | path data with the `B`/`b` bearing commands of an old SVG 2 draft (dropped from the spec): not implemented (80 pixels differ (largest difference 255); first at (45,48): #ffffff, the reference has #7f7fff) |
| distance/path-length-css-overrides-presentation-attribute.tentative.svg | limit (CSS path-length) | the CSS `path-length` property (tentative test) is not implemented, only the `pathLength` attribute (2392 pixels differ (largest difference 255); first at (22,17): #ffffff, the reference has #7f7f7f) |
| distance/path-length-css-property.tentative.svg | limit (CSS path-length) | the CSS `path-length` property (tentative test) is not implemented, only the `pathLength` attribute (4744 pixels differ (largest difference 128); first at (20,17): #bfbfbf, the reference has #7f7f7f) |
| distance/path-length-css-zero.tentative.svg | limit (CSS path-length) | the CSS `path-length` property (tentative test) is not implemented, only the `pathLength` attribute (2024 pixels differ (largest difference 255); first at (5,5): #ffffff, the reference has #000000) |
| distance/path-length-css-zoom.tentative.svg | limit (CSS path-length) | the CSS `path-length` property (tentative test) is not implemented, only the `pathLength` attribute (4744 pixels differ (largest difference 128); first at (20,17): #bfbfbf, the reference has #7f7f7f) |
| property/d-none.svg | limit (CSS d) | the CSS `d` property (`d: path(...)`) is not implemented, only the `d` attribute (10000 pixels differ (largest difference 255); first at (0,0): #ff0000, the reference has #008000) |
| property/priority.svg | limit (CSS d) | the CSS `d` property (`d: path(...)`) is not implemented, only the `d` attribute (1120 pixels differ (largest difference 255); first at (20,8): #ffffff, the reference has #7f7fff) |

## 通过

- distance/pathlength-path-negative.svg
- distance/pathlength-path-zero.svg
- distance/pathlength-path.svg
- distance/pathlength-rect.svg
- error-handling/render-until-error.svg
