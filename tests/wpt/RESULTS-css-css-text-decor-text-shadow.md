# WPT `css/css-text-decor/text-shadow`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/css-text-decor/text-shadow`，用时 0.1 秒。

- 通过 **16 / 18**（88.9%）
- bug 类失败：**1**
- 跳过：无

失败分类：bug (open) 1，limit (emoji) 1

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| decorations-multiple-zorder.html | bug (open) | text-shadow with several decorations: the shadows of the underline are not drawn (text-shadow applies to the text only); not investigated in 2c-2 (240 pixels differ (largest difference 255); first at (132,20): #ff0000, the reference has #008000) |
| text-shadow-emoji-transparent.html | limit (emoji) | color emoji are step 2d (800 pixels differ (largest difference 255); first at (300,8): #ffffff, the reference has #000000) |

## 通过

- basic-negcoord.html
- basic-opacity-near-zero.html
- basic-opacity-zero.html
- basic-opacity.html
- basic.html
- blur.html
- color-inherit.html
- color-parserorder.html
- currentcolor.html
- lineoverflow.html
- multiple-noblur.html
- padding-decoration.html
- quirks-decor-noblur.html
- standards-decor-noblur-2.html
- standards-decor-noblur.html
- textindent.html
