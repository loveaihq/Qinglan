# WPT `html/rendering/widgets`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt html/rendering/widgets`，用时 0.1 秒。

- 通过 **11 / 19**（57.9%）
- bug 类失败：**0**
- 跳过：no reftest 2

失败分类：limit (button layout) 5，limit (pseudo-elements on controls) 2，limit (switch) 1

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| button-layout/anonymous-button-content-box.html | limit (button layout) | the content of a button is not an anonymous block that is centered in a taller button (html rendering 15.5.1) (27193 pixels differ (largest difference 255); first at (0,0): #ffffff, the reference has #000000) |
| button-layout/centering-001.html | limit (button layout) | button content is not centered vertically inside a button taller than its content (1014 pixels differ (largest difference 239); first at (96,11): #000000, the reference has #efefef) |
| button-layout/centering-002.html | limit (button layout) | same as centering-001 (1014 pixels differ (largest difference 239); first at (96,11): #000000, the reference has #efefef) |
| button-layout/centering-003.html | limit (button layout) | same as centering-001 (horizontal and vertical) (2454 pixels differ (largest difference 239); first at (16,11): #000000, the reference has #efefef) |
| button-layout/scrollable-button-centering.html | limit (button layout) | a scrollable button: its content is not centered when it fits, and does not scroll (6410 pixels differ (largest difference 255); first at (78,8): #ffffff, the reference has #e7e7e7) |
| input-checkbox-no-centering.html | limit (pseudo-elements on controls) | ::before on an <input> with appearance: none is not generated (inputs are replaced boxes without children) (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #008000) |
| input-checkbox-switch-rtl.tentative.html | limit (switch) | input[switch] (a toggle look) is not drawn (renders like html/rendering/widgets/input-checkbox-switch-rtl-notref.html, which it must not) |
| input-radio-no-centering.html | limit (pseudo-elements on controls) | same as input-checkbox-no-centering (10000 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #008000) |

## 通过

- button-layout/block-in-inline.html
- button-layout/display-none-or-contents.html
- button-layout/input-type-button-newline-2.html
- button-layout/input-type-button-newline.html
- button-layout/propagate-text-decoration.html
- input-checkbox-disabled-checked.html
- input-checkbox-zero-size.html
- input-password-background-suppresses-appearance.html
- input-radio-disabled-checked.html
- text-control-flow-root.html
- textarea-multicol.html
