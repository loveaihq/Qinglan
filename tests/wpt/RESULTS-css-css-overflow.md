# WPT `css/css-overflow`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/css-overflow`，用时 0.1 秒。

- 通过 **18 / 23**（78.3%）
- bug 类失败：**0**
- 跳过：无

失败分类：limit (replaced boxes) 4，limit (scrollable overflow) 1

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| overflow-img-border-radius.html | limit (replaced boxes) | same as overflow-img: no overflow clip of an <img> content with object-fit/object-position (4572 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #008001) |
| overflow-img-object-position.html | limit (replaced boxes) | same as overflow-img: object-position is not applied (10000 pixels differ (largest difference 255); first at (8,8): #008001, the reference has #ffffff) |
| overflow-img-svg.html | limit (replaced boxes) | same as overflow-img (an SVG image) (4700 pixels differ (largest difference 255); first at (20,8): #3f7f7f, the reference has #0000ff) |
| overflow-img.html | limit (replaced boxes) | overflow on a replaced <img>: the picture is not clipped or offset by its own overflow, object-fit/object-position on a box bigger than the image (jpg with EXIF orientation too) are not applied; same for the next three (4400 pixels differ (largest difference 255); first at (20,8): #80c001, the reference has #008001) |
| overflow-negative-margin.html | limit (scrollable overflow) | the end padding is added to the scrollable overflow when the content's border box reaches into it; Chrome measures the in-flow margin box, so a negative bottom margin that pulls the child back into the content box shows no bar there (CSS-overflow-3 2.2) (542 pixels differ (largest difference 138); first at (100,48): #8a8a8a, the reference has #008000) |

## 通过

- clip-001.html
- clip-002.html
- clip-004.html
- clip-005.html
- clip-006.html
- clip-007.html
- float-with-relpos-and-transform.html
- overflow-body-propagation-002.html
- overflow-body-propagation-003.html
- overflow-body-propagation-007.html
- overflow-body-propagation-008.html
- overflow-body-propagation-009.html
- overflow-body-propagation-012.html
- overflow-clip-transform-001.html
- overflow-clipped-transparent-border-clip.html
- overflow-does-not-apply-to-inline-box.html
- overflow-inline-block-with-opacity.html
- scrollbar-empty-001.html
