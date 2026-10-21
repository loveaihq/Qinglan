# WPT `svg/shapes`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt svg/shapes`，用时 0.2 秒。

- 通过 **23 / 25**（92.0%）
- bug 类失败：**1**
- 跳过：无

失败分类：bug (open) 1，limit (zoom) 1

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| reftests/pathlength-002.svg | bug (open) | dashes along a circle with `pathLength` (the 0.25 dash scaled by 4 against the explicit 39.27): 1.5k pixels differ by up to 64/255, the fuzzy limit is 1; the phase of the dashes drifts, not looked into (1506 pixels differ (largest difference 64); first at (199,229): #e4e4e4, the reference has #ffffff) |
| reftests/pathlength-005.html | limit (zoom) | the CSS `zoom` property is not implemented (6880 pixels differ (largest difference 255); first at (13,57): #ffffff, the reference has #f9fcf9) |

## 通过

- circle-01.svg
- ellipse-01.svg
- ellipse-02.svg
- ellipse-03.svg
- ellipse-04.svg
- ellipse-05.svg
- ellipse-06.svg
- ellipse-07.svg
- ellipse-08.svg
- ellipse-09.svg
- line-dasharray.svg
- rect-01.svg
- rect-02.svg
- rect-03.svg
- rect-04.svg
- rect-05.svg
- rect-rx-set-by-css.svg
- reftests/disabled-shapes-01.svg
- reftests/ellipse-auto-rx-clip-path.html
- reftests/ellipse-auto-rx-css-clip-path.html
- reftests/ellipse-auto-rx-percent-clip-path.html
- reftests/pathlength-001.svg
- reftests/pathlength-004a.svg
