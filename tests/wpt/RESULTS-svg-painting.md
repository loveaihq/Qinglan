# WPT `svg/painting`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt svg/painting`，用时 0.1 秒。

- 通过 **9 / 11**（81.8%）
- bug 类失败：**0**
- 跳过：无

失败分类：limit (external use) 2

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| reftests/gradient-external-reference.svg | limit (external use) | `use`, `fill` or `stroke` refer to another file (`file.svg#id`): only same-document references are followed (2e-1) (10000 pixels differ (largest difference 128); first at (0,0): #000000, the reference has #008000) |
| reftests/pattern-external-reference.svg | limit (external use) | `use`, `fill` or `stroke` refer to another file (`file.svg#id`): only same-document references are followed (2e-1) (10000 pixels differ (largest difference 128); first at (0,0): #000000, the reference has #008000) |

## 通过

- currentColor-override-pserver-fallback.svg
- inner-outer-svg-overflow-auto.html
- negative-dashoffset-odd-dasharray.html
- reftests/fallback-001.svg
- reftests/fallback-002.svg
- reftests/large-transform-001.html
- reftests/percentage-attribute.svg
- reftests/percentage.svg
- reftests/small-nested-viewbox.html
