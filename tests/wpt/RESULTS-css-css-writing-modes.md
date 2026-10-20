# WPT `css/css-writing-modes`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/css-writing-modes`，用时 8.8 秒。

- 通过 **439 / 843**（52.1%）
- bug 类失败：**388**
- 跳过：无

失败分类：bug (vertical writing) 388，limit (plaintext paragraphs) 3，limit (test platform glyph width) 12，step 2e 1

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| abs-pos-border-offset-001.html | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (13800 pixels differ (largest difference 203); first at (40,1): #ffc0cb, the reference has #add8e6) |
| abs-pos-border-offset-002.html | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (25200 pixels differ (largest difference 203); first at (19,1): #ffa500, the reference has #ffc0cb) |
| abs-pos-border-offset-003.html | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (2610 pixels differ (largest difference 255); first at (103,18): #0000ff, the reference has #ffffff) |
| abs-pos-non-replaced-icb-vlr-005.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (10800 pixels differ (largest difference 255); first at (0,68): #008000, the reference has #ffffff) |
| abs-pos-non-replaced-icb-vlr-013.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (10800 pixels differ (largest difference 255); first at (0,68): #008000, the reference has #ffffff) |
| abs-pos-non-replaced-icb-vrl-004.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (10800 pixels differ (largest difference 255); first at (0,68): #008000, the reference has #ffffff) |
| abs-pos-non-replaced-icb-vrl-012.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (10800 pixels differ (largest difference 255); first at (0,68): #008000, the reference has #ffffff) |
| abs-pos-non-replaced-vlr-007.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-011.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-013.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-017.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-019.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-023.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-025.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-031.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-037.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-043.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-049.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-055.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-061.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-065.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6400 pixels differ (largest difference 255); first at (168,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-067.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-071.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6400 pixels differ (largest difference 255); first at (168,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-073.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-079.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-085.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-091.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-093.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-095.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-097.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (18791 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-107.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-113.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-115.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-117.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-123.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-125.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #008000, the reference has #ffffff) |
| abs-pos-non-replaced-vlr-129.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,231): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-131.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-139.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-147.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-155.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-157.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (168,231): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-161.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-163.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-165.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (168,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-171.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-173.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #008000, the reference has #ffffff) |
| abs-pos-non-replaced-vlr-179.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-185.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6400 pixels differ (largest difference 255); first at (168,231): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-187.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-189.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6400 pixels differ (largest difference 255); first at (88,231): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-193.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6400 pixels differ (largest difference 255); first at (88,231): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-195.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-197.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6400 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-203.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-205.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,231): #008000, the reference has #ffffff) |
| abs-pos-non-replaced-vlr-209.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-211.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-213.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (168,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-219.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-221.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,231): #008000, the reference has #ffffff) |
| abs-pos-non-replaced-vlr-225.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,231): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vlr-227.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vlr-229.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (168,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-006.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-010.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-012.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-016.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-018.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-022.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-024.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-030.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-036.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-042.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-048.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-054.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-060.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-064.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6400 pixels differ (largest difference 255); first at (168,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-066.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-070.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6400 pixels differ (largest difference 255); first at (168,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-072.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-078.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-084.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-090.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-092.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (168,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-094.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-096.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (19631 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-106.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-112.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-114.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-116.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (168,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-122.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-124.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (168,151): #008000, the reference has #ffffff) |
| abs-pos-non-replaced-vrl-128.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,231): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-130.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-132.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-138.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-146.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-154.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-156.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (168,231): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-160.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-162.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-164.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (168,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-170.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-172.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (168,151): #008000, the reference has #ffffff) |
| abs-pos-non-replaced-vrl-178.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-180.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-184.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6400 pixels differ (largest difference 255); first at (168,231): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-186.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-188.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6400 pixels differ (largest difference 255); first at (88,231): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-192.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6400 pixels differ (largest difference 255); first at (88,231): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-194.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-196.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (6400 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-202.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-208.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-210.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-212.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (168,151): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-218.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-220.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,231): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-224.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,231): #ff0000, the reference has #008000) |
| abs-pos-non-replaced-vrl-226.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| abs-pos-non-replaced-vrl-228.xht | bug (vertical writing) | position: absolute inside vertical frames (static position / insets of vertical, rtl and mixed-mode containing blocks, absolute boxes in the vertical root): the logical frame resolves them like horizontal ones; not diagnosed test by test (12800 pixels differ (largest difference 255); first at (88,151): #ff0000, the reference has #008000) |
| available-size-001.html | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (800 pixels differ (largest difference 255); first at (8,84): #ff0000, the reference has #008000) |
| available-size-003.html | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (800 pixels differ (largest difference 255); first at (8,84): #ff0000, the reference has #008000) |
| available-size-005.html | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (400 pixels differ (largest difference 255); first at (8,84): #ff0000, the reference has #008000) |
| available-size-011.html | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (1536 pixels differ (largest difference 255); first at (24,80): #ffffff, the reference has #000000) |
| available-size-012.html | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (800 pixels differ (largest difference 255); first at (8,84): #ff0000, the reference has #008000) |
| available-size-013.html | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (800 pixels differ (largest difference 255); first at (8,84): #ff0000, the reference has #008000) |
| available-size-014.html | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (400 pixels differ (largest difference 255); first at (8,84): #ff0000, the reference has #008000) |
| available-size-018.html | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (800 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| available-size-020.html | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (21690 pixels differ (largest difference 255); first at (171,87): #ffffff, the reference has #0000ff) |
| background-position-vrl-018.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (121500 pixels differ (largest difference 255); first at (424,0): #ff0000, the reference has #ffffff) |
| background-position-vrl-020.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (121500 pixels differ (largest difference 255); first at (424,0): #ff0000, the reference has #ffffff) |
| background-position-vrl-022.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (121500 pixels differ (largest difference 255); first at (424,0): #ff0000, the reference has #ffffff) |
| baseline-inline-non-replaced-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (4000 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| baseline-inline-non-replaced-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (4500 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| baseline-inline-replaced-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (4709 pixels differ (largest difference 255); first at (8,48): #ff0000, the reference has #ffffff) |
| baseline-inline-replaced-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (7623 pixels differ (largest difference 255); first at (106,48): #ff0000, the reference has #ffffff) |
| baseline-with-orthogonal-flow-001.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (1536 pixels differ (largest difference 255); first at (8,56): #ffffff, the reference has #000000) |
| bidi-isolate-override-001.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (1152 pixels differ (largest difference 255); first at (96,54): #ffffff, the reference has #000000) |
| bidi-isolate-override-002.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (1152 pixels differ (largest difference 255); first at (96,54): #ffffff, the reference has #000000) |
| bidi-isolate-override-003.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (2304 pixels differ (largest difference 255); first at (96,54): #000000, the reference has #ffffff) |
| bidi-isolate-override-004.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (2304 pixels differ (largest difference 255); first at (96,54): #000000, the reference has #ffffff) |
| bidi-override-001.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (1152 pixels differ (largest difference 255); first at (96,54): #ffffff, the reference has #000000) |
| bidi-override-002.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (1152 pixels differ (largest difference 255); first at (96,54): #ffffff, the reference has #000000) |
| bidi-override-003.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (2304 pixels differ (largest difference 255); first at (96,54): #000000, the reference has #ffffff) |
| bidi-override-004.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (2304 pixels differ (largest difference 255); first at (96,54): #000000, the reference has #ffffff) |
| bidi-plaintext-br-001.html | limit (plaintext paragraphs) | unicode-bidi: plaintext starts a new bidi paragraph, with its own base direction, after every forced break (<br> or a preserved newline); the engine analyses a block as one paragraph (decisions.md 2d-2a) (7168 pixels differ (largest difference 255); first at (9,93): #000000, the reference has #ffffff) |
| block-flow-direction-slr-043.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (151200 pixels differ (largest difference 255); first at (8,8): #0000ff, the reference has #ffffff) |
| block-flow-direction-slr-047.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (12000 pixels differ (largest difference 255); first at (412,28): #0000ff, the reference has #ffff00) |
| block-flow-direction-slr-048.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (12000 pixels differ (largest difference 255); first at (412,28): #0000ff, the reference has #ffff00) |
| block-flow-direction-slr-050.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (12000 pixels differ (largest difference 255); first at (48,28): #0000ff, the reference has #ffff00) |
| block-flow-direction-slr-054.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (29600 pixels differ (largest difference 255); first at (28,8): #ffff00, the reference has #0000ff) |
| block-flow-direction-slr-055.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (29600 pixels differ (largest difference 255); first at (28,8): #ffff00, the reference has #0000ff) |
| block-flow-direction-slr-056.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (29600 pixels differ (largest difference 255); first at (28,8): #ffff00, the reference has #0000ff) |
| block-flow-direction-slr-062.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (218160 pixels differ (largest difference 255); first at (8,8): #0000ff, the reference has #ffffff) |
| block-flow-direction-slr-063.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (151200 pixels differ (largest difference 255); first at (8,8): #0000ff, the reference has #ffffff) |
| block-flow-direction-slr-066.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (27658 pixels differ (largest difference 255); first at (8,8): #0000ff, the reference has #ffffff) |
| block-flow-direction-srl-061.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (91280 pixels differ (largest difference 255); first at (52,0): #ffff00, the reference has #ffffff) |
| block-flow-direction-srl-065.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (7149 pixels differ (largest difference 255); first at (422,8): #ffffff, the reference has #fff5c6) |
| block-flow-direction-vlr-022.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (90560 pixels differ (largest difference 255); first at (428,8): #0000ff, the reference has #ffffff) |
| block-flow-direction-vrl-021.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (91280 pixels differ (largest difference 255); first at (52,0): #ffff00, the reference has #ffffff) |
| block-flow-direction-vrl-025.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (7149 pixels differ (largest difference 255); first at (422,8): #ffffff, the reference has #fff5c6) |
| block-flow-direction-vrl-026.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (1600 pixels differ (largest difference 255); first at (39,65): #0000ff, the reference has #ffc0cb) |
| block-override-001.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (2304 pixels differ (largest difference 255); first at (72,54): #000000, the reference has #ffffff) |
| block-override-004.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (6912 pixels differ (largest difference 255); first at (72,54): #000000, the reference has #ffffff) |
| block-override-isolate-001.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (2304 pixels differ (largest difference 255); first at (72,54): #000000, the reference has #ffffff) |
| block-override-isolate-004.html | limit (test platform glyph width) | the test is made for the narrow ezra web font and its box is 10em wide: with 1em glyphs the lines wrap at other places than in the reference; with the boxes widened to 60em (a scratch copy, checked 2026-10-20) this test passes (6912 pixels differ (largest difference 255); first at (72,54): #000000, the reference has #ffffff) |
| block-plaintext-004.html | limit (plaintext paragraphs) | unicode-bidi: plaintext starts a new bidi paragraph, with its own base direction, after every forced break (<br> or a preserved newline); the engine analyses a block as one paragraph (decisions.md 2d-2a) (8064 pixels differ (largest difference 255); first at (24,102): #000000, the reference has #ffffff) |
| block-plaintext-006.html | limit (plaintext paragraphs) | unicode-bidi: plaintext starts a new bidi paragraph, with its own base direction, after every forced break (<br> or a preserved newline); the engine analyses a block as one paragraph (decisions.md 2d-2a) (39052 pixels differ (largest difference 255); first at (264,110): #000000, the reference has #ffffff) |
| border-slice-001.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (1800 pixels differ (largest difference 255); first at (54,101): #ff00ff, the reference has #ffffff) |
| border-vlr-007.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (2800 pixels differ (largest difference 255); first at (214,83): #0000ff, the reference has #ffffff) |
| border-vrl-006.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (600 pixels differ (largest difference 255); first at (111,103): #ffffff, the reference has #ffff00) |
| box-offsets-rel-pos-vlr-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (30311 pixels differ (largest difference 255); first at (533,0): #ff0000, the reference has #ffffff) |
| box-offsets-rel-pos-vrl-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (58357 pixels differ (largest difference 255); first at (383,0): #ff0000, the reference has #ffffff) |
| central-baseline-alignment-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3780 pixels differ (largest difference 255); first at (50,64): #ffa500, the reference has #ffffff) |
| central-baseline-alignment-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3780 pixels differ (largest difference 255); first at (50,64): #ffa500, the reference has #ffffff) |
| clearance-calculations-vrl-008.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (10000 pixels differ (largest difference 255); first at (8,108): #ff0000, the reference has #ffffff) |
| clip-rect-vlr-003.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (7500 pixels differ (largest difference 255); first at (58,71): #ff0000, the reference has #ffffff) |
| clip-rect-vlr-005.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (7500 pixels differ (largest difference 255); first at (8,71): #ff0000, the reference has #ffffff) |
| clip-rect-vlr-007.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (7500 pixels differ (largest difference 255); first at (8,71): #ff0000, the reference has #ffffff) |
| clip-rect-vlr-009.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (7500 pixels differ (largest difference 255); first at (8,71): #ff0000, the reference has #ffffff) |
| clip-rect-vlr-011.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (13161 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| clip-rect-vlr-013.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (13711 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| clip-rect-vlr-015.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (13161 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| clip-rect-vlr-017.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (13711 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| clip-rect-vrl-002.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (7500 pixels differ (largest difference 255); first at (58,71): #ff0000, the reference has #ffffff) |
| clip-rect-vrl-004.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (7500 pixels differ (largest difference 255); first at (8,71): #ff0000, the reference has #ffffff) |
| clip-rect-vrl-006.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (7500 pixels differ (largest difference 255); first at (8,71): #ff0000, the reference has #ffffff) |
| clip-rect-vrl-008.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (7500 pixels differ (largest difference 255); first at (8,71): #ff0000, the reference has #ffffff) |
| clip-rect-vrl-010.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (15403 pixels differ (largest difference 255); first at (422,8): #ffffff, the reference has #008000) |
| clip-rect-vrl-012.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (15403 pixels differ (largest difference 255); first at (472,8): #ffffff, the reference has #008000) |
| clip-rect-vrl-014.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (15403 pixels differ (largest difference 255); first at (543,8): #ff0000, the reference has #ffffff) |
| clip-rect-vrl-016.xht | bug (vertical writing) | clip on an absolutely positioned box in a vertical frame: not diagnosed (clip: rect() sides are physical, the frame permutes sides) (15403 pixels differ (largest difference 255); first at (543,8): #ff0000, the reference has #ffffff) |
| different-block-flow-dir-001.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (13456 pixels differ (largest difference 255); first at (8,0): #ff0000, the reference has #ffffff) |
| different-block-flow-dir-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (13456 pixels differ (largest difference 255); first at (8,0): #ff0000, the reference has #ffffff) |
| float-clear-vlr-007.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (2500 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| float-clear-vlr-009.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (2500 pixels differ (largest difference 255); first at (58,64): #ff0000, the reference has #008000) |
| float-clear-vrl-006.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (2500 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| float-clear-vrl-008.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (2500 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| float-contiguous-vlr-007.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (27078 pixels differ (largest difference 255); first at (121,8): #ffffff, the reference has #fff5c6) |
| float-contiguous-vlr-009.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (24664 pixels differ (largest difference 255); first at (121,8): #ffffff, the reference has #fff5c6) |
| float-contiguous-vlr-013.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (2000 pixels differ (largest difference 255); first at (692,52): #ff0000, the reference has #ffffff) |
| float-contiguous-vrl-006.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (28447 pixels differ (largest difference 255); first at (592,8): #ffffff, the reference has #fff5c6) |
| float-contiguous-vrl-008.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (28437 pixels differ (largest difference 255); first at (592,8): #ffffff, the reference has #fff5c6) |
| float-contiguous-vrl-012.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (2000 pixels differ (largest difference 255); first at (692,52): #ff0000, the reference has #ffffff) |
| float-in-htb-in-vrl.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| float-lft-orthog-htb-in-vlr-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (24704 pixels differ (largest difference 255); first at (56,16): #0000ff, the reference has #ffa500) |
| float-lft-orthog-htb-in-vrl-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (14336 pixels differ (largest difference 255); first at (272,16): #0000ff, the reference has #ffffff) |
| float-rgt-orthog-htb-in-vlr-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (27776 pixels differ (largest difference 255); first at (16,40): #ffffff, the reference has #0000ff) |
| float-rgt-orthog-htb-in-vrl-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (15104 pixels differ (largest difference 255); first at (784,40): #ffa500, the reference has #0000ff) |
| float-rgt-orthog-vlr-in-htb-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (9792 pixels differ (largest difference 255); first at (744,16): #0000ff, the reference has #ffffff) |
| float-rgt-orthog-vrl-in-htb-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (9792 pixels differ (largest difference 255); first at (744,16): #0000ff, the reference has #ffffff) |
| float-shrink-to-fit-vlr-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (10000 pixels differ (largest difference 255); first at (8,64): #ff0000, the reference has #008000) |
| float-shrink-to-fit-vlr-005.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (28576 pixels differ (largest difference 255); first at (8,0): #ff0000, the reference has #ffffff) |
| float-shrink-to-fit-vlr-007.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (58944 pixels differ (largest difference 255); first at (8,0): #ff0000, the reference has #ffffff) |
| float-shrink-to-fit-vrl-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3712 pixels differ (largest difference 255); first at (8,0): #ff0000, the reference has #ffffff) |
| float-shrink-to-fit-vrl-004.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (28576 pixels differ (largest difference 255); first at (8,0): #ff0000, the reference has #ffffff) |
| float-shrink-to-fit-vrl-006.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (58944 pixels differ (largest difference 255); first at (8,0): #ff0000, the reference has #ffffff) |
| float-shrink-to-fit-vrl-vlr-016.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (58688 pixels differ (largest difference 255); first at (8,0): #ff0000, the reference has #ffffff) |
| full-width-001.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (renders like css/css-writing-modes/reference/full-width-001-nofullwidth-notref.html, which it must not) |
| full-width-003.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (renders like css/css-writing-modes/reference/full-width-002-notcu-notref.html, which it must not) |
| horizontal-rule-vlr-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (11601 pixels differ (largest difference 255); first at (121,0): #ffffff, the reference has #fff5c6) |
| horizontal-rule-vrl-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (11427 pixels differ (largest difference 255); first at (498,0): #ffffff, the reference has #008000) |
| inline-block-alignment-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3780 pixels differ (largest difference 255); first at (80,64): #ffa500, the reference has #ffffff) |
| inline-block-alignment-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3780 pixels differ (largest difference 255); first at (200,64): #ffa500, the reference has #ffffff) |
| inline-block-alignment-004.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3780 pixels differ (largest difference 255); first at (80,64): #ffa500, the reference has #ffffff) |
| inline-block-alignment-005.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3780 pixels differ (largest difference 255); first at (200,64): #ffa500, the reference has #ffffff) |
| inline-block-alignment-006.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3780 pixels differ (largest difference 255); first at (80,64): #ff00ff, the reference has #ffffff) |
| inline-block-alignment-007.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3780 pixels differ (largest difference 255); first at (200,64): #0000ff, the reference has #ffffff) |
| inline-block-alignment-orthogonal-vlr-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (900 pixels differ (largest difference 255); first at (8,199): #ffa500, the reference has #ffffff) |
| inline-block-alignment-orthogonal-vlr-005.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (900 pixels differ (largest difference 255); first at (8,199): #ffa500, the reference has #ffffff) |
| inline-block-alignment-orthogonal-vrl-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (900 pixels differ (largest difference 255); first at (8,199): #ffa500, the reference has #ffffff) |
| inline-block-alignment-orthogonal-vrl-004.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (900 pixels differ (largest difference 255); first at (8,199): #ffa500, the reference has #ffffff) |
| inline-block-alignment-slr-009.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (17460 pixels differ (largest difference 255); first at (200,64): #008080, the reference has #ffffff) |
| inline-block-alignment-srl-008.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (3780 pixels differ (largest difference 255); first at (80,64): #ff00ff, the reference has #ffffff) |
| inline-replaced-vlr-005.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (14691 pixels differ (largest difference 255); first at (108,82): #ffffff, the reference has #ccccff) |
| inline-replaced-vrl-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (2206 pixels differ (largest difference 255); first at (66,64): #ffffff, the reference has #000000) |
| inline-replaced-vrl-004.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (23209 pixels differ (largest difference 255); first at (78,82): #ffffff, the reference has #ccccff) |
| line-box-direction-slr-043.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (12000 pixels differ (largest difference 255); first at (48,28): #0000ff, the reference has #ffff00) |
| line-box-direction-slr-047.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (12000 pixels differ (largest difference 255); first at (412,28): #0000ff, the reference has #ffff00) |
| line-box-direction-slr-048.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (12000 pixels differ (largest difference 255); first at (412,28): #0000ff, the reference has #ffff00) |
| line-box-direction-slr-050.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (12000 pixels differ (largest difference 255); first at (48,28): #0000ff, the reference has #ffff00) |
| line-box-direction-slr-053.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (29600 pixels differ (largest difference 255); first at (28,8): #ffff00, the reference has #0000ff) |
| line-box-direction-slr-054.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (29600 pixels differ (largest difference 255); first at (28,8): #ffff00, the reference has #0000ff) |
| line-box-direction-slr-060.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (30800 pixels differ (largest difference 255); first at (428,8): #0000ff, the reference has #ffffff) |
| line-box-direction-srl-059.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (31600 pixels differ (largest difference 255); first at (428,8): #0000ff, the reference has #ffffff) |
| line-box-direction-vlr-020.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (31600 pixels differ (largest difference 255); first at (428,8): #0000ff, the reference has #ffffff) |
| line-box-direction-vrl-019.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (31600 pixels differ (largest difference 255); first at (428,8): #0000ff, the reference has #ffffff) |
| margin-collapse-vlr-023.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (5000 pixels differ (largest difference 255); first at (58,33): #0000ff, the reference has #ffa500) |
| margin-collapse-vrl-022.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (5000 pixels differ (largest difference 255); first at (58,33): #0000ff, the reference has #ffa500) |
| nested-orthogonal-001.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (10000 pixels differ (largest difference 255); first at (8,64): #ffffff, the reference has #008000) |
| normal-flow-overconstrained-vlr-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (7210 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| normal-flow-overconstrained-vlr-005.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (7210 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| normal-flow-overconstrained-vrl-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (5610 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| normal-flow-overconstrained-vrl-004.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (5610 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| overconstrained-rel-pos-ltr-left-right-vlr-005.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| overconstrained-rel-pos-ltr-left-right-vrl-004.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (8511 pixels differ (largest difference 255); first at (362,8): #ff0000, the reference has #008000) |
| overconstrained-rel-pos-ltr-top-bottom-vlr-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| overconstrained-rel-pos-ltr-top-bottom-vrl-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| overconstrained-rel-pos-rtl-left-right-vlr-009.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (6911 pixels differ (largest difference 255); first at (347,512): #008000, the reference has #ffffff) |
| overconstrained-rel-pos-rtl-left-right-vrl-008.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (8471 pixels differ (largest difference 255); first at (362,512): #ff0000, the reference has #008000) |
| overconstrained-rel-pos-rtl-top-bottom-vlr-007.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (12191 pixels differ (largest difference 255); first at (278,432): #ff0000, the reference has #008000) |
| overconstrained-rel-pos-rtl-top-bottom-vrl-006.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (12151 pixels differ (largest difference 255); first at (442,432): #ff0000, the reference has #008000) |
| padding-vlr-005.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (1500 pixels differ (largest difference 255); first at (214,83): #0000ff, the reference has #ffffff) |
| padding-vrl-004.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (600 pixels differ (largest difference 255); first at (111,103): #0000ff, the reference has #ffff00) |
| percent-padding-vlr-005.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (5590 pixels differ (largest difference 255); first at (214,80): #0000ff, the reference has #ffffff) |
| percent-padding-vlr-007.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (12385 pixels differ (largest difference 255); first at (177,80): #ffffff, the reference has #0000ff) |
| percent-padding-vrl-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (600 pixels differ (largest difference 255); first at (111,103): #0000ff, the reference has #ffff00) |
| percent-padding-vrl-004.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (1890 pixels differ (largest difference 255); first at (214,80): #0000ff, the reference has #ffffff) |
| percent-padding-vrl-006.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (12385 pixels differ (largest difference 255); first at (177,80): #ffffff, the reference has #0000ff) |
| replaced-content-image-004.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (300 pixels differ (largest difference 255); first at (8,8): #ffffff, the reference has #0000ff) |
| scrollbar-vertical-rl.html | step 2e | uses svg, scrolling, sticky or form controls (1800 pixels differ (largest difference 255); first at (0,9): #0000ff, the reference has #ffffff) |
| sizing-orthog-prct-htb-in-vlr-002.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (50168 pixels differ (largest difference 255); first at (506,8): #0000ff, the reference has #ffffff) |
| sizing-orthog-prct-htb-in-vlr-006.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (54932 pixels differ (largest difference 255); first at (406,8): #0000ff, the reference has #ffffff) |
| sizing-orthog-prct-htb-in-vrl-002.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (66088 pixels differ (largest difference 255); first at (0,8): #0000ff, the reference has #ffffff) |
| sizing-orthog-prct-htb-in-vrl-006.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (74388 pixels differ (largest difference 255); first at (0,8): #0000ff, the reference has #ffffff) |
| sizing-orthog-prct-vlr-in-htb-001.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (123441 pixels differ (largest difference 255); first at (136,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-prct-vlr-in-htb-002.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (53349 pixels differ (largest difference 255); first at (100,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-prct-vlr-in-htb-003.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (64912 pixels differ (largest difference 255); first at (136,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-prct-vlr-in-htb-004.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (65008 pixels differ (largest difference 255); first at (100,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-prct-vlr-in-htb-005.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (133371 pixels differ (largest difference 255); first at (52,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-prct-vlr-in-htb-006.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (62095 pixels differ (largest difference 255); first at (8,0): #0000ff, the reference has #ffffff) |
| sizing-orthog-prct-vlr-in-htb-007.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (64064 pixels differ (largest difference 255); first at (52,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-prct-vlr-in-htb-008.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (66130 pixels differ (largest difference 255); first at (8,0): #0000ff, the reference has #ffffff) |
| sizing-orthog-prct-vrl-in-htb-001.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (117153 pixels differ (largest difference 255); first at (8,136): #0000ff, the reference has #ffffff) |
| sizing-orthog-prct-vrl-in-htb-002.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (50534 pixels differ (largest difference 255); first at (8,100): #0000ff, the reference has #ffffff) |
| sizing-orthog-prct-vrl-in-htb-003.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (63344 pixels differ (largest difference 255); first at (136,86): #ffffff, the reference has #0000ff) |
| sizing-orthog-prct-vrl-in-htb-004.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (62736 pixels differ (largest difference 255); first at (100,86): #ffffff, the reference has #0000ff) |
| sizing-orthog-prct-vrl-in-htb-005.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (131750 pixels differ (largest difference 255); first at (8,52): #0000ff, the reference has #ffffff) |
| sizing-orthog-prct-vrl-in-htb-006.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (64012 pixels differ (largest difference 255); first at (8,0): #0000ff, the reference has #ffffff) |
| sizing-orthog-prct-vrl-in-htb-007.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (64431 pixels differ (largest difference 255); first at (8,52): #0000ff, the reference has #ffffff) |
| sizing-orthog-prct-vrl-in-htb-008.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (65444 pixels differ (largest difference 255); first at (8,0): #0000ff, the reference has #ffffff) |
| sizing-orthog-vlr-in-htb-001.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (51821 pixels differ (largest difference 255); first at (136,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-vlr-in-htb-003.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (10776 pixels differ (largest difference 255); first at (136,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-vlr-in-htb-004.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (54237 pixels differ (largest difference 255); first at (100,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-vlr-in-htb-006.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (10824 pixels differ (largest difference 255); first at (100,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-vlr-in-htb-007.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (61804 pixels differ (largest difference 255); first at (136,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-vlr-in-htb-008.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (23560 pixels differ (largest difference 255); first at (136,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-vlr-in-htb-009.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (10776 pixels differ (largest difference 255); first at (136,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-vlr-in-htb-010.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (61453 pixels differ (largest difference 255); first at (100,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-vlr-in-htb-011.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (24616 pixels differ (largest difference 255); first at (100,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-vlr-in-htb-013.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (56266 pixels differ (largest difference 255); first at (52,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-vlr-in-htb-015.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (10568 pixels differ (largest difference 255); first at (52,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-vlr-in-htb-016.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (134184 pixels differ (largest difference 255); first at (8,0): #0000ff, the reference has #ffffff) |
| sizing-orthog-vlr-in-htb-018.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (7209 pixels differ (largest difference 255); first at (8,0): #0000ff, the reference has #ffffff) |
| sizing-orthog-vlr-in-htb-019.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (59910 pixels differ (largest difference 255); first at (52,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-vlr-in-htb-020.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (25704 pixels differ (largest difference 255); first at (52,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-vlr-in-htb-021.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (10568 pixels differ (largest difference 255); first at (52,8): #ffffff, the reference has #0000ff) |
| sizing-orthog-vlr-in-htb-022.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (63091 pixels differ (largest difference 255); first at (8,0): #0000ff, the reference has #ffffff) |
| sizing-orthog-vlr-in-htb-023.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (18479 pixels differ (largest difference 255); first at (8,0): #0000ff, the reference has #ffffff) |
| sizing-orthog-vlr-in-htb-024.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (7209 pixels differ (largest difference 255); first at (8,0): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-001.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (49718 pixels differ (largest difference 255); first at (8,136): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-003.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (7448 pixels differ (largest difference 255); first at (8,136): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-004.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (51746 pixels differ (largest difference 255); first at (8,100): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-006.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (7448 pixels differ (largest difference 255); first at (8,100): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-007.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (59628 pixels differ (largest difference 255); first at (8,136): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-008.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (14680 pixels differ (largest difference 255); first at (8,136): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-009.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (7448 pixels differ (largest difference 255); first at (8,136): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-010.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (59795 pixels differ (largest difference 255); first at (8,100): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-011.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (15688 pixels differ (largest difference 255); first at (8,100): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-012.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (7448 pixels differ (largest difference 255); first at (8,100): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-013.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (57072 pixels differ (largest difference 255); first at (8,52): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-015.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (7448 pixels differ (largest difference 255); first at (8,52): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-016.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (108880 pixels differ (largest difference 255); first at (8,0): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-018.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (7448 pixels differ (largest difference 255); first at (8,0): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-019.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (61148 pixels differ (largest difference 255); first at (8,52): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-020.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (17032 pixels differ (largest difference 255); first at (8,52): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-021.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (7448 pixels differ (largest difference 255); first at (8,52): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-022.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (63292 pixels differ (largest difference 255); first at (8,0): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-023.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (17440 pixels differ (largest difference 255); first at (8,0): #0000ff, the reference has #ffffff) |
| sizing-orthog-vrl-in-htb-024.xht | bug (vertical writing) | orthogonal-flow sizing: the reference uses a vertical root element (html writing-mode with absolutely positioned children) which the root frame lays out differently (root block size and padding); the test side matches the spec text (dump checked for sizing-orthog-vrl-in-htb-001) (7448 pixels differ (largest difference 255); first at (8,0): #0000ff, the reference has #ffffff) |
| sizing-percentages-replaced-orthogonal-001.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (40000 pixels differ (largest difference 255); first at (615,79): #ff0000, the reference has #00ff00) |
| slr-alongside-vlr-floats.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3200 pixels differ (largest difference 255); first at (353,8): #ffa500, the reference has #ffffff) |
| srl-alongside-vrl-floats.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3200 pixels differ (largest difference 255); first at (441,8): #ffa500, the reference has #ffffff) |
| tcy-white-space-processing-001.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (156 pixels differ (largest difference 255); first at (192,87): #ffffff, the reference has #0000ff) |
| text-baseline-slr-009.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (10260 pixels differ (largest difference 255); first at (56,64): #ff00ff, the reference has #ffffff) |
| text-baseline-srl-008.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (3780 pixels differ (largest difference 255); first at (50,64): #ff00ff, the reference has #ffffff) |
| text-baseline-vlr-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3780 pixels differ (largest difference 255); first at (50,64): #ffa500, the reference has #ffffff) |
| text-baseline-vlr-005.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3780 pixels differ (largest difference 255); first at (50,64): #ffa500, the reference has #ffffff) |
| text-baseline-vlr-007.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3780 pixels differ (largest difference 255); first at (50,64): #ff00ff, the reference has #ffffff) |
| text-baseline-vrl-002.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3780 pixels differ (largest difference 255); first at (50,64): #ffa500, the reference has #ffffff) |
| text-baseline-vrl-004.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3780 pixels differ (largest difference 255); first at (50,64): #ffa500, the reference has #ffffff) |
| text-baseline-vrl-006.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3780 pixels differ (largest difference 255); first at (50,64): #ff00ff, the reference has #ffffff) |
| text-combine-upright-break-inside-001a.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3072 pixels differ (largest difference 255); first at (187,48): #ffffff, the reference has #000000) |
| text-combine-upright-layout-rules-001.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (5000 pixels differ (largest difference 255); first at (228,174): #ffffff, the reference has #000000) |
| text-combine-upright-shadow.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (1660 pixels differ (largest difference 255); first at (8,164): #000000, the reference has #ffffff) |
| text-combine-upright-sideways-001.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3840 pixels differ (largest difference 255); first at (8,128): #ffffff, the reference has #000000) |
| text-combine-upright-sideways-002.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (2720 pixels differ (largest difference 255); first at (8,128): #ffffff, the reference has #000000) |
| text-combine-upright-value-all-002.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (4372 pixels differ (largest difference 255); first at (88,144): #ffffff, the reference has #000000) |
| text-combine-upright-value-all-003.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (25600 pixels differ (largest difference 255); first at (88,224): #000000, the reference has #ffffff) |
| text-combine-upright-value-digits2-002.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (6400 pixels differ (largest difference 255); first at (88,224): #000000, the reference has #ffffff) |
| text-combine-upright-value-digits3-001.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (6400 pixels differ (largest difference 255); first at (88,224): #000000, the reference has #ffffff) |
| text-combine-upright-value-digits3-002.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (12800 pixels differ (largest difference 255); first at (88,224): #000000, the reference has #ffffff) |
| text-combine-upright-value-digits4-001.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (12800 pixels differ (largest difference 255); first at (88,224): #000000, the reference has #ffffff) |
| text-combine-upright-value-digits4-002.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (19200 pixels differ (largest difference 255); first at (88,224): #000000, the reference has #ffffff) |
| text-indent-vlr-011.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| text-indent-vlr-013.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| text-indent-vlr-015.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| text-indent-vlr-017.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (6871 pixels differ (largest difference 255); first at (120,8): #ffffff, the reference has #fff5c6) |
| text-indent-vrl-010.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| text-indent-vrl-012.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| text-indent-vrl-014.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| text-indent-vrl-016.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (8511 pixels differ (largest difference 255); first at (642,8): #ffffff, the reference has #fff5c6) |
| text-orientation-016.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (6800 pixels differ (largest difference 255); first at (20,79): #ffff00, the reference has #0000ff) |
| text-shadow-orientation-upright-001.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (40000 pixels differ (largest difference 255); first at (108,8): #ffffff, the reference has #800080) |
| text-shadow-sideways-001.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (4864 pixels differ (largest difference 255); first at (28,19): #ffa500, the reference has #c0c0c0) |
| three-levels-of-orthogonal-flows.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (36864 pixels differ (largest difference 255); first at (424,64): #ff69b4, the reference has #ffff00) |
| two-levels-of-orthogonal-flows-fixed.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (5725 pixels differ (largest difference 255); first at (5,11): #ffff00, the reference has #ffffff) |
| two-levels-of-orthogonal-flows-percentage.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (6400 pixels differ (largest difference 255); first at (24,128): #ffffff, the reference has #ffff00) |
| two-levels-of-orthogonal-flows.html | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (3328 pixels differ (largest difference 255); first at (24,112): #ffffff, the reference has #ffff00) |
| vertical-alignment-003.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (1800 pixels differ (largest difference 255); first at (38,94): #ffa500, the reference has #ffffff) |
| vertical-alignment-005.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (1800 pixels differ (largest difference 255); first at (68,124): #ffffff, the reference has #ffa500) |
| vertical-alignment-007.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (1800 pixels differ (largest difference 255); first at (68,124): #ffa500, the reference has #ffffff) |
| vertical-alignment-009.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (1800 pixels differ (largest difference 255); first at (38,94): #ffffff, the reference has #ffa500) |
| vertical-alignment-slr-029.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (5400 pixels differ (largest difference 255); first at (38,80): #ffffff, the reference has #ffa500) |
| vertical-alignment-slr-031.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (1800 pixels differ (largest difference 255); first at (68,64): #ffa500, the reference has #ffffff) |
| vertical-alignment-slr-033.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (1800 pixels differ (largest difference 255); first at (98,64): #ffa500, the reference has #ffffff) |
| vertical-alignment-slr-035.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (5400 pixels differ (largest difference 255); first at (68,80): #0000ff, the reference has #ffffff) |
| vertical-alignment-slr-041.xht | bug (vertical writing) | sideways-lr (counter-clockwise glyphs, inline direction bottom to top) is parsed but laid out as vertical-lr with sideways glyphs (14400 pixels differ (largest difference 255); first at (108,64): #ffffff, the reference has #0000ff) |
| vertical-alignment-vlr-023.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (800 pixels differ (largest difference 255); first at (78,164): #0000ff, the reference has #ffffff) |
| vertical-alignment-vlr-025.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (800 pixels differ (largest difference 255); first at (78,164): #0000ff, the reference has #ffffff) |
| vertical-alignment-vlr-027.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (14400 pixels differ (largest difference 255); first at (108,64): #ffffff, the reference has #0000ff) |
| vertical-alignment-vrl-022.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (800 pixels differ (largest difference 255); first at (158,164): #0000ff, the reference has #ffffff) |
| vertical-alignment-vrl-024.xht | bug (vertical writing) | vertical-frame difference not diagnosed (see decisions.md 2d-2b, known gaps) (800 pixels differ (largest difference 255); first at (158,164): #0000ff, the reference has #ffffff) |
| wm-propagation-body-032.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (7149 pixels differ (largest difference 255); first at (422,8): #ffffff, the reference has #fff5c6) |
| wm-propagation-body-034.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (7149 pixels differ (largest difference 255); first at (422,8): #ffffff, the reference has #fff5c6) |
| wm-propagation-body-035.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (27358 pixels differ (largest difference 255); first at (8,8): #008080, the reference has #ffffff) |
| wm-propagation-body-036.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (7149 pixels differ (largest difference 255); first at (422,8): #ffffff, the reference has #fff5c6) |
| wm-propagation-body-038.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (7149 pixels differ (largest difference 255); first at (422,8): #ffffff, the reference has #fff5c6) |
| wm-propagation-body-039.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (27358 pixels differ (largest difference 255); first at (8,8): #008080, the reference has #ffffff) |
| wm-propagation-body-041.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (7203 pixels differ (largest difference 255); first at (229,8): #ffffff, the reference has #fff5c6) |
| wm-propagation-body-042.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (11124 pixels differ (largest difference 255); first at (274,0): #000000, the reference has #ffffff) |
| wm-propagation-body-043.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (27358 pixels differ (largest difference 255); first at (8,8): #008080, the reference has #ffffff) |
| wm-propagation-body-044.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (4736 pixels differ (largest difference 255); first at (0,196): #ffffff, the reference has #000000) |
| wm-propagation-body-045.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (7149 pixels differ (largest difference 255); first at (422,8): #ffffff, the reference has #fff5c6) |
| wm-propagation-body-046.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (7149 pixels differ (largest difference 255); first at (422,8): #ffffff, the reference has #fff5c6) |
| wm-propagation-body-047.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (24350 pixels differ (largest difference 255); first at (462,0): #000000, the reference has #ffffff) |
| wm-propagation-body-049.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (19260 pixels differ (largest difference 255); first at (290,0): #000000, the reference has #ffffff) |
| wm-propagation-body-050.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (7203 pixels differ (largest difference 255); first at (229,8): #ffffff, the reference has #fff5c6) |
| wm-propagation-body-051.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (27358 pixels differ (largest difference 255); first at (8,8): #008080, the reference has #ffffff) |
| wm-propagation-body-053.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (7149 pixels differ (largest difference 255); first at (422,8): #ffffff, the reference has #fff5c6) |
| wm-propagation-body-054.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (10885 pixels differ (largest difference 255); first at (485,0): #000000, the reference has #ffffff) |
| wm-propagation-body-055.html | bug (vertical writing) | body writing mode propagated to the root (layout dump of wm-propagation-body-032 puts the square upper right as asserted); the pixel difference with the reference is not diagnosed (7149 pixels differ (largest difference 255); first at (422,8): #ffffff, the reference has #fff5c6) |

## 通过

- abs-pos-non-replaced-icb-vlr-003.xht
- abs-pos-non-replaced-icb-vlr-011.xht
- abs-pos-non-replaced-icb-vrl-002.xht
- abs-pos-non-replaced-icb-vrl-010.xht
- abs-pos-non-replaced-vlr-003.xht
- abs-pos-non-replaced-vlr-005.xht
- abs-pos-non-replaced-vlr-009.xht
- abs-pos-non-replaced-vlr-015.xht
- abs-pos-non-replaced-vlr-021.xht
- abs-pos-non-replaced-vlr-027.xht
- abs-pos-non-replaced-vlr-029.xht
- abs-pos-non-replaced-vlr-033.xht
- abs-pos-non-replaced-vlr-035.xht
- abs-pos-non-replaced-vlr-039.xht
- abs-pos-non-replaced-vlr-041.xht
- abs-pos-non-replaced-vlr-045.xht
- abs-pos-non-replaced-vlr-047.xht
- abs-pos-non-replaced-vlr-051.xht
- abs-pos-non-replaced-vlr-053.xht
- abs-pos-non-replaced-vlr-057.xht
- abs-pos-non-replaced-vlr-059.xht
- abs-pos-non-replaced-vlr-063.xht
- abs-pos-non-replaced-vlr-069.xht
- abs-pos-non-replaced-vlr-075.xht
- abs-pos-non-replaced-vlr-077.xht
- abs-pos-non-replaced-vlr-081.xht
- abs-pos-non-replaced-vlr-083.xht
- abs-pos-non-replaced-vlr-087.xht
- abs-pos-non-replaced-vlr-089.xht
- abs-pos-non-replaced-vlr-103.xht
- abs-pos-non-replaced-vlr-105.xht
- abs-pos-non-replaced-vlr-109.xht
- abs-pos-non-replaced-vlr-111.xht
- abs-pos-non-replaced-vlr-119.xht
- abs-pos-non-replaced-vlr-121.xht
- abs-pos-non-replaced-vlr-127.xht
- abs-pos-non-replaced-vlr-133.xht
- abs-pos-non-replaced-vlr-135.xht
- abs-pos-non-replaced-vlr-137.xht
- abs-pos-non-replaced-vlr-141.xht
- abs-pos-non-replaced-vlr-143.xht
- abs-pos-non-replaced-vlr-145.xht
- abs-pos-non-replaced-vlr-149.xht
- abs-pos-non-replaced-vlr-151.xht
- abs-pos-non-replaced-vlr-153.xht
- abs-pos-non-replaced-vlr-159.xht
- abs-pos-non-replaced-vlr-167.xht
- abs-pos-non-replaced-vlr-169.xht
- abs-pos-non-replaced-vlr-175.xht
- abs-pos-non-replaced-vlr-177.xht
- abs-pos-non-replaced-vlr-181.xht
- abs-pos-non-replaced-vlr-183.xht
- abs-pos-non-replaced-vlr-191.xht
- abs-pos-non-replaced-vlr-199.xht
- abs-pos-non-replaced-vlr-201.xht
- abs-pos-non-replaced-vlr-207.xht
- abs-pos-non-replaced-vlr-215.xht
- abs-pos-non-replaced-vlr-217.xht
- abs-pos-non-replaced-vlr-223.xht
- abs-pos-non-replaced-vrl-002.xht
- abs-pos-non-replaced-vrl-004.xht
- abs-pos-non-replaced-vrl-008.xht
- abs-pos-non-replaced-vrl-014.xht
- abs-pos-non-replaced-vrl-020.xht
- abs-pos-non-replaced-vrl-026.xht
- abs-pos-non-replaced-vrl-028.xht
- abs-pos-non-replaced-vrl-032.xht
- abs-pos-non-replaced-vrl-034.xht
- abs-pos-non-replaced-vrl-038.xht
- abs-pos-non-replaced-vrl-040.xht
- abs-pos-non-replaced-vrl-044.xht
- abs-pos-non-replaced-vrl-046.xht
- abs-pos-non-replaced-vrl-050.xht
- abs-pos-non-replaced-vrl-052.xht
- abs-pos-non-replaced-vrl-056.xht
- abs-pos-non-replaced-vrl-058.xht
- abs-pos-non-replaced-vrl-062.xht
- abs-pos-non-replaced-vrl-068.xht
- abs-pos-non-replaced-vrl-074.xht
- abs-pos-non-replaced-vrl-076.xht
- abs-pos-non-replaced-vrl-080.xht
- abs-pos-non-replaced-vrl-082.xht
- abs-pos-non-replaced-vrl-086.xht
- abs-pos-non-replaced-vrl-088.xht
- abs-pos-non-replaced-vrl-102.xht
- abs-pos-non-replaced-vrl-104.xht
- abs-pos-non-replaced-vrl-108.xht
- abs-pos-non-replaced-vrl-110.xht
- abs-pos-non-replaced-vrl-118.xht
- abs-pos-non-replaced-vrl-120.xht
- abs-pos-non-replaced-vrl-126.xht
- abs-pos-non-replaced-vrl-134.xht
- abs-pos-non-replaced-vrl-136.xht
- abs-pos-non-replaced-vrl-140.xht
- abs-pos-non-replaced-vrl-142.xht
- abs-pos-non-replaced-vrl-144.xht
- abs-pos-non-replaced-vrl-148.xht
- abs-pos-non-replaced-vrl-150.xht
- abs-pos-non-replaced-vrl-152.xht
- abs-pos-non-replaced-vrl-158.xht
- abs-pos-non-replaced-vrl-166.xht
- abs-pos-non-replaced-vrl-168.xht
- abs-pos-non-replaced-vrl-174.xht
- abs-pos-non-replaced-vrl-176.xht
- abs-pos-non-replaced-vrl-182.xht
- abs-pos-non-replaced-vrl-190.xht
- abs-pos-non-replaced-vrl-198.xht
- abs-pos-non-replaced-vrl-200.xht
- abs-pos-non-replaced-vrl-204.xht
- abs-pos-non-replaced-vrl-206.xht
- abs-pos-non-replaced-vrl-214.xht
- abs-pos-non-replaced-vrl-216.xht
- abs-pos-non-replaced-vrl-222.xht
- available-size-002.html
- available-size-004.html
- available-size-006.html
- available-size-007.html
- available-size-008.html
- available-size-009.html
- available-size-010.html
- available-size-015.html
- available-size-016.html
- available-size-017.html
- available-size-019.html
- baseline-inline-non-replaced-004.xht
- baseline-inline-non-replaced-005.xht
- bidi-embed-001.html
- bidi-embed-002.html
- bidi-embed-003.html
- bidi-embed-004.html
- bidi-embed-005.html
- bidi-embed-006.html
- bidi-embed-007.html
- bidi-embed-008.html
- bidi-embed-009.html
- bidi-embed-010.html
- bidi-embed-011.html
- bidi-isolate-001.html
- bidi-isolate-002.html
- bidi-isolate-003.html
- bidi-isolate-004.html
- bidi-isolate-005.html
- bidi-isolate-006.html
- bidi-isolate-007.html
- bidi-isolate-008.html
- bidi-isolate-009.html
- bidi-isolate-010.html
- bidi-isolate-011.html
- bidi-isolate-override-005.html
- bidi-isolate-override-006.html
- bidi-isolate-override-007.html
- bidi-isolate-override-008.html
- bidi-isolate-override-009.html
- bidi-isolate-override-010.html
- bidi-isolate-override-011.html
- bidi-isolate-override-012.html
- bidi-normal-001.html
- bidi-normal-002.html
- bidi-normal-003.html
- bidi-normal-004.html
- bidi-normal-005.html
- bidi-normal-006.html
- bidi-normal-007.html
- bidi-normal-008.html
- bidi-normal-009.html
- bidi-normal-010.html
- bidi-normal-011.html
- bidi-override-005.html
- bidi-override-006.html
- bidi-override-007.html
- bidi-override-008.html
- bidi-override-009.html
- bidi-override-010.html
- bidi-override-011.html
- bidi-override-012.html
- bidi-plaintext-001.html
- bidi-plaintext-002.html
- bidi-plaintext-003.html
- bidi-plaintext-004.html
- bidi-plaintext-005.html
- bidi-plaintext-006.html
- bidi-plaintext-007.html
- bidi-plaintext-008.html
- bidi-plaintext-009.html
- bidi-plaintext-010.html
- bidi-plaintext-011.html
- bidi-unset-001.html
- bidi-unset-002.html
- bidi-unset-003.html
- bidi-unset-004.html
- bidi-unset-005.html
- bidi-unset-006.html
- bidi-unset-007.html
- bidi-unset-008.html
- bidi-unset-009.html
- bidi-unset-010.html
- block-embed-001.html
- block-embed-002.html
- block-embed-003.html
- block-flow-direction-004.xht
- block-flow-direction-htb-001.xht
- block-flow-direction-srl-042.xht
- block-flow-direction-srl-045.xht
- block-flow-direction-srl-046.xht
- block-flow-direction-srl-049.xht
- block-flow-direction-srl-051.xht
- block-flow-direction-srl-052.xht
- block-flow-direction-srl-053.xht
- block-flow-direction-srl-064.xht
- block-flow-direction-vlr-003.xht
- block-flow-direction-vlr-007.xht
- block-flow-direction-vlr-008.xht
- block-flow-direction-vlr-010.xht
- block-flow-direction-vlr-014.xht
- block-flow-direction-vlr-015.xht
- block-flow-direction-vlr-016.xht
- block-flow-direction-vlr-023.xht
- block-flow-direction-vrl-002.xht
- block-flow-direction-vrl-005.xht
- block-flow-direction-vrl-006.xht
- block-flow-direction-vrl-009.xht
- block-flow-direction-vrl-011.xht
- block-flow-direction-vrl-012.xht
- block-flow-direction-vrl-013.xht
- block-flow-direction-vrl-024.xht
- block-override-002.html
- block-override-003.html
- block-override-isolate-002.html
- block-override-isolate-003.html
- block-plaintext-001.html
- block-plaintext-002.html
- block-plaintext-003.html
- clearance-calculations-vrl-002.xht
- clearance-calculations-vrl-004.xht
- clearance-calculations-vrl-006.xht
- direction-upright-001.html
- direction-vlr-003.xht
- direction-vlr-005.xht
- direction-vrl-002.xht
- direction-vrl-004.xht
- float-clear-vlr-003.xht
- float-clear-vlr-005.xht
- float-clear-vrl-002.xht
- float-clear-vrl-004.xht
- float-contiguous-vlr-003.xht
- float-contiguous-vlr-005.xht
- float-contiguous-vlr-011.xht
- float-contiguous-vrl-002.xht
- float-contiguous-vrl-004.xht
- float-contiguous-vrl-010.xht
- float-lft-orthog-vlr-in-htb-002.xht
- float-lft-orthog-vrl-in-htb-002.xht
- float-shrink-to-fit-vlr-009.xht
- float-shrink-to-fit-vrl-008.xht
- float-vlr-003.xht
- float-vlr-005.xht
- float-vlr-007.xht
- float-vlr-009.xht
- float-vlr-011.xht
- float-vlr-013.xht
- float-vlr-014.html
- float-vrl-002.xht
- float-vrl-004.xht
- float-vrl-006.xht
- float-vrl-008.xht
- float-vrl-010.xht
- float-vrl-012.xht
- full-width-002.html
- height-width-inline-non-replaced-vlr-003.xht
- height-width-inline-non-replaced-vrl-002.xht
- horizontal-rule-vlr-005.xht
- horizontal-rule-vrl-004.xht
- inline-box-border-vlr-001.html
- inline-box-orthogonal-child-with-margins.html
- inline-replaced-vlr-003.xht
- line-box-direction-htb-001.xht
- line-box-direction-srl-042.xht
- line-box-direction-srl-045.xht
- line-box-direction-srl-046.xht
- line-box-direction-srl-049.xht
- line-box-direction-srl-051.xht
- line-box-direction-srl-052.xht
- line-box-direction-vlr-003.xht
- line-box-direction-vlr-007.xht
- line-box-direction-vlr-008.xht
- line-box-direction-vlr-010.xht
- line-box-direction-vlr-013.xht
- line-box-direction-vlr-014.xht
- line-box-direction-vrl-002.xht
- line-box-direction-vrl-005.xht
- line-box-direction-vrl-006.xht
- line-box-direction-vrl-009.xht
- line-box-direction-vrl-011.xht
- line-box-direction-vrl-012.xht
- line-box-height-vlr-003.xht
- line-box-height-vlr-005.xht
- line-box-height-vlr-007.xht
- line-box-height-vlr-009.xht
- line-box-height-vlr-011.xht
- line-box-height-vlr-013.xht
- line-box-height-vlr-021.xht
- line-box-height-vlr-023.xht
- line-box-height-vrl-002.xht
- line-box-height-vrl-004.xht
- line-box-height-vrl-006.xht
- line-box-height-vrl-008.xht
- line-box-height-vrl-010.xht
- line-box-height-vrl-012.xht
- margin-collapse-vlr-003.xht
- margin-collapse-vlr-009.xht
- margin-collapse-vlr-011.xht
- margin-collapse-vlr-015.xht
- margin-collapse-vlr-017.xht
- margin-collapse-vlr-025.xht
- margin-collapse-vlr-031.xht
- margin-collapse-vlr-035.xht
- margin-collapse-vlr-037.xht
- margin-collapse-vrl-002.xht
- margin-collapse-vrl-008.xht
- margin-collapse-vrl-010.xht
- margin-collapse-vrl-014.xht
- margin-collapse-vrl-016.xht
- margin-collapse-vrl-024.xht
- margin-collapse-vrl-030.xht
- margin-collapse-vrl-034.xht
- margin-collapse-vrl-036.xht
- margin-vlr-003.xht
- margin-vrl-002.xht
- mongolian-orientation-001.html
- mongolian-orientation-002.html
- mongolian-span-001.html
- outline-inline-block-vrl-006.html
- outline-inline-vlr-006.html
- outline-inline-vrl-006.html
- percent-margin-vlr-003.xht
- percent-margin-vlr-005.xht
- percent-margin-vlr-007.xht
- percent-margin-vrl-002.xht
- percent-margin-vrl-004.xht
- percent-margin-vrl-006.xht
- percent-padding-vlr-003.xht
- relpos-inline-overflowing-block-vrl.html
- sizing-orthog-htb-in-vlr-006.xht
- sizing-orthog-htb-in-vlr-010.xht
- sizing-orthog-htb-in-vlr-011.xht
- sizing-orthog-htb-in-vlr-012.xht
- sizing-orthog-htb-in-vlr-016.xht
- sizing-orthog-htb-in-vlr-018.xht
- sizing-orthog-htb-in-vlr-022.xht
- sizing-orthog-htb-in-vlr-023.xht
- sizing-orthog-htb-in-vlr-024.xht
- sizing-orthog-htb-in-vrl-006.xht
- sizing-orthog-htb-in-vrl-010.xht
- sizing-orthog-htb-in-vrl-011.xht
- sizing-orthog-htb-in-vrl-012.xht
- sizing-orthog-htb-in-vrl-016.xht
- sizing-orthog-htb-in-vrl-018.xht
- sizing-orthog-htb-in-vrl-022.xht
- sizing-orthog-htb-in-vrl-023.xht
- sizing-orthog-htb-in-vrl-024.xht
- sizing-orthog-prct-htb-in-vlr-004.xht
- sizing-orthog-prct-htb-in-vlr-008.xht
- sizing-orthog-prct-htb-in-vrl-004.xht
- sizing-orthog-prct-htb-in-vrl-008.xht
- sizing-orthog-vlr-in-htb-012.xht
- sizing-orthogonal-percentage-margin-001.html
- sizing-orthogonal-percentage-margin-002.html
- sizing-orthogonal-percentage-margin-003.html
- sizing-orthogonal-percentage-margin-004.html
- sizing-orthogonal-percentage-margin-005.html
- sizing-orthogonal-percentage-margin-006.html
- sizing-orthogonal-percentage-margin-007.html
- sizing-orthogonal-percentage-margin-008.html
- tcy-white-space-processing-002.html
- tcy-white-space-processing-003.html
- text-align-vlr-003.xht
- text-align-vlr-005.xht
- text-align-vlr-007.xht
- text-align-vlr-009.xht
- text-align-vlr-011.xht
- text-align-vlr-013.xht
- text-align-vlr-015.xht
- text-align-vlr-017.xht
- text-align-vlr-019.xht
- text-align-vrl-002.xht
- text-align-vrl-004.xht
- text-align-vrl-006.xht
- text-align-vrl-008.xht
- text-align-vrl-010.xht
- text-align-vrl-012.xht
- text-align-vrl-014.xht
- text-align-vrl-016.xht
- text-align-vrl-018.xht
- text-combine-upright-break-inside-001.html
- text-combine-upright-gen-con-001.html
- text-combine-upright-inherit-all-001.html
- text-combine-upright-letter-spacing-001.html
- text-combine-upright-rtl-001.html
- text-combine-upright-rtl-002.html
- text-combine-upright-value-digits2-003.html
- text-combine-upright-value-digits3-003.html
- text-combine-upright-value-digits4-003.html
- text-combine-upright-value-none-001.html
- text-indent-vlr-003.xht
- text-indent-vlr-005.xht
- text-indent-vlr-007.xht
- text-indent-vlr-009.xht
- text-indent-vrl-002.xht
- text-indent-vrl-004.xht
- text-indent-vrl-006.xht
- text-indent-vrl-008.xht
- text-orientation-020.html
- text-orientation-021.html
- text-orientation-initial.html
- text-orientation-upright-directionality-001.html
- vertical-alignment-002.xht
- vertical-alignment-004.xht
- vertical-alignment-006.xht
- vertical-alignment-008.xht
- vertical-alignment-srl-028.xht
- vertical-alignment-srl-030.xht
- vertical-alignment-srl-032.xht
- vertical-alignment-srl-034.xht
- vertical-alignment-srl-040.xht
- vertical-alignment-vrl-026.xht
- vlr-text-orientation-sideways-alongside-vlr-floats.html
- vrl-text-orientation-sideways-alongside-vrl-floats.html
- wm-propagation-body-033.html
- wm-propagation-body-037.html
- wm-propagation-body-040.html
- wm-propagation-body-048.html
- wm-propagation-body-052.html
- writing-mode-horizontal-001l.html
- writing-mode-horizontal-001r.html
- writing-mode-initial.html
- writing-mode-vertical-lr-002.xht
- writing-mode-vertical-rl-001.xht
- writing-mode-vertical-rl-002.xht
- writing-mode-vertical-rl-003.htm
