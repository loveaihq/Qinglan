# WPT `svg/struct`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt svg/struct`，用时 0.3 秒。

- 通过 **18 / 32**（56.2%）
- bug 类失败：**0**
- 跳过：无

失败分类：limit (external use) 10，limit (use shadow tree) 3，not planned 1

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| reftests/use-external-html-resource-with-content-type-svg.html | limit (external use) | `use`, `fill` or `stroke` refer to another file (`file.svg#id`): only same-document references are followed (2e-1) (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #008000) |
| reftests/use-external-html-resource-with-doctype.html | limit (external use) | `use`, `fill` or `stroke` refer to another file (`file.svg#id`): only same-document references are followed (2e-1) (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #008000) |
| reftests/use-external-html-resource.html | limit (external use) | `use`, `fill` or `stroke` refer to another file (`file.svg#id`): only same-document references are followed (2e-1) (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #008000) |
| reftests/use-external-resource-nested-image.html | limit (external use) | `use`, `fill` or `stroke` refer to another file (`file.svg#id`): only same-document references are followed (2e-1) (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #008000) |
| reftests/use-external-resource-nested-symbol.html | limit (external use) | `use`, `fill` or `stroke` refer to another file (`file.svg#id`): only same-document references are followed (2e-1) (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #008000) |
| reftests/use-external-resource-no-svg-root.html | limit (external use) | `use`, `fill` or `stroke` refer to another file (`file.svg#id`): only same-document references are followed (2e-1) (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #008000) |
| reftests/use-external-resource-target-pseudo-001.html | limit (external use) | `use`, `fill` or `stroke` refer to another file (`file.svg#id`): only same-document references are followed (2e-1) (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #008000) |
| reftests/use-external-resource-target-pseudo-002.html | limit (external use) | `use`, `fill` or `stroke` refer to another file (`file.svg#id`): only same-document references are followed (2e-1) (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #008000) |
| reftests/use-external-svg-resource-no-fragment-id.html | limit (external use) | `use`, `fill` or `stroke` refer to another file (`file.svg#id`): only same-document references are followed (2e-1) (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #008000) |
| reftests/use-external-use.svg | not planned | uses a feature layer 2 does not do (multi-column, shapes, animation, text-wrap: balance, aspect-ratio ...) (10000 pixels differ (largest difference 255); first at (0,0): #ffffff, the reference has #008000) |
| reftests/use-inheritance-001.svg | limit (use shadow tree) | SVG 2 `use` shadow tree: style rules match the clone below the `use` element (`use rect`, `:nth-child(of S)`); the engine matches the original element where it stands and inherits from the `use` (10000 pixels differ (largest difference 255); first at (0,0): #ff0000, the reference has #008000) |
| reftests/use-inheritance-nth-child-of.svg | limit (use shadow tree) | SVG 2 `use` shadow tree: style rules match the clone below the `use` element (`use rect`, `:nth-child(of S)`); the engine matches the original element where it stands and inherits from the `use` (10000 pixels differ (largest difference 255); first at (0,0): #ff0000, the reference has #008000) |
| reftests/use-inheritance-nth-last-child-of.svg | limit (use shadow tree) | SVG 2 `use` shadow tree: style rules match the clone below the `use` element (`use rect`, `:nth-child(of S)`); the engine matches the original element where it stands and inherits from the `use` (10000 pixels differ (largest difference 255); first at (0,0): #ff0000, the reference has #008000) |
| reftests/use-same-origin.svg | limit (external use) | `use`, `fill` or `stroke` refer to another file (`file.svg#id`): only same-document references are followed (2e-1) (10000 pixels differ (largest difference 255); first at (0,0): #ffffff, the reference has #008000) |

## 通过

- reftests/gradient-in-symbol.svg
- reftests/inner-svg-css-transform.svg
- reftests/inner-svg-rotate-transform.svg
- reftests/inner-svg-transform-and-viewbox.svg
- reftests/inner-svg-transform.svg
- reftests/nested-svg-through-display-contents.svg
- reftests/outer-svg-transform.svg
- reftests/use-data-url.tentative.svg
- reftests/use-encoding.svg
- reftests/use-svg-dimensions-override-001.svg
- reftests/use-svg-dimensions-override-002.svg
- reftests/use-svg-inline-css.svg
- reftests/use-symbol-dimensions-override-001.svg
- reftests/use-symbol-dimensions-override-002.svg
- reftests/use-symbol-display-none.svg
- reftests/use-symbol-inline-css-001.svg
- reftests/use-symbol-inline-css-002.svg
- reftests/use-symbol-inline-css-003.svg
