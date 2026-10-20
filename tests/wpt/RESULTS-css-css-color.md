# WPT `css/css-color`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/css-color`，用时 0.1 秒。

- 通过 **11 / 12**（91.7%）
- bug 类失败：**0**
- 跳过：无

失败分类：limit (inline opacity) 1

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| inline-opacity-float-child.html | limit (inline opacity) | opacity on an inline box groups its inline content only, not the floats and blocks inside it (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |

## 通过

- opacity-overlapping-letters.html
- t32-opacity-basic-0.0-a.xht
- t32-opacity-basic-0.6-a.xht
- t32-opacity-basic-1.0-a.xht
- t32-opacity-clamping-0.0-b.xht
- t32-opacity-clamping-1.0-b.xht
- t32-opacity-offscreen-b.xht
- t32-opacity-offscreen-multiple-boxes-1-c.xht
- t32-opacity-offscreen-multiple-boxes-2-c.xht
- t32-opacity-offscreen-with-alpha-c.xht
- t32-opacity-zorder-c.xht
