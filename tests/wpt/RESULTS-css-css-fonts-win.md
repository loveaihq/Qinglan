# WPT `css/css-fonts`

Windows 平台（DirectWrite，真字体），800×600，`qinglan-cli wpt css/css-fonts`，用时 0.5 秒。

- 通过 **32 / 51**（62.7%）
- bug 类失败：**3**
- 跳过：无

失败分类：bug (U+2028/U+2029 not drawn as spaces) 1，bug (default OpenType features) 1，bug (strut of a line whose first font has no space) 1，limit (default font) 1，limit (font-stretch) 12，limit (rex unit) 1，limit (ric unit) 1，limit (system font weight matching) 1

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| font-default-01.html | bug (default OpenType features) | not investigated in 2d-1: the font's calt/liga/clig substitutions do not happen for the accented characters; the words are shaped by DirectWrite without explicit features and in bulk, which may drop them (1169 pixels differ (largest difference 255); first at (333,105): #f9f9f9, the reference has #ffffff) |
| font-face-unicode-range-2.html | bug (strut of a line whose first font has no space) | not investigated in 2d-1: lines are 85 px apart in the test and 105 px in the reference; first-available-font-001 to 007 pass, so the rule of 5.1 is in; the reference's own family lists decide the difference (17931 pixels differ (largest difference 255); first at (191,56): #f9f9f9, the reference has #ffffff) |
| font-stretch-02.html | limit (font-stretch) | the font-stretch property is not in the style engine: FontQuery has no stretch, so only normal (100%) is ever asked for; the @font-face font-stretch descriptor is read and ranked (fonts::face::rank_faces) but cannot be selected. Later step (8878 pixels differ (largest difference 255); first at (145,129): #ffffff, the reference has #f6f6f6) |
| font-stretch-03.html | limit (font-stretch) | the font-stretch property is not in the style engine: FontQuery has no stretch, so only normal (100%) is ever asked for; the @font-face font-stretch descriptor is read and ranked (fonts::face::rank_faces) but cannot be selected. Later step (8878 pixels differ (largest difference 255); first at (145,129): #ffffff, the reference has #f6f6f6) |
| font-stretch-04.html | limit (font-stretch) | the font-stretch property is not in the style engine: FontQuery has no stretch, so only normal (100%) is ever asked for; the @font-face font-stretch descriptor is read and ranked (fonts::face::rank_faces) but cannot be selected. Later step (8878 pixels differ (largest difference 255); first at (145,129): #ffffff, the reference has #f6f6f6) |
| font-stretch-05.html | limit (font-stretch) | the font-stretch property is not in the style engine: FontQuery has no stretch, so only normal (100%) is ever asked for; the @font-face font-stretch descriptor is read and ranked (fonts::face::rank_faces) but cannot be selected. Later step (8878 pixels differ (largest difference 255); first at (145,129): #ffffff, the reference has #f6f6f6) |
| font-stretch-07.html | limit (font-stretch) | the font-stretch property is not in the style engine: FontQuery has no stretch, so only normal (100%) is ever asked for; the @font-face font-stretch descriptor is read and ranked (fonts::face::rank_faces) but cannot be selected. Later step (8878 pixels differ (largest difference 255); first at (145,129): #ffffff, the reference has #f6f6f6) |
| font-stretch-08.html | limit (font-stretch) | the font-stretch property is not in the style engine: FontQuery has no stretch, so only normal (100%) is ever asked for; the @font-face font-stretch descriptor is read and ranked (fonts::face::rank_faces) but cannot be selected. Later step (8878 pixels differ (largest difference 255); first at (145,129): #ffffff, the reference has #f6f6f6) |
| font-stretch-09.html | limit (font-stretch) | the font-stretch property is not in the style engine: FontQuery has no stretch, so only normal (100%) is ever asked for; the @font-face font-stretch descriptor is read and ranked (fonts::face::rank_faces) but cannot be selected. Later step (8878 pixels differ (largest difference 255); first at (145,129): #ffffff, the reference has #f6f6f6) |
| font-stretch-10.html | limit (font-stretch) | the font-stretch property is not in the style engine: FontQuery has no stretch, so only normal (100%) is ever asked for; the @font-face font-stretch descriptor is read and ranked (fonts::face::rank_faces) but cannot be selected. Later step (8878 pixels differ (largest difference 255); first at (145,129): #ffffff, the reference has #f6f6f6) |
| font-stretch-11.html | limit (font-stretch) | the font-stretch property is not in the style engine: FontQuery has no stretch, so only normal (100%) is ever asked for; the @font-face font-stretch descriptor is read and ranked (fonts::face::rank_faces) but cannot be selected. Later step (8878 pixels differ (largest difference 255); first at (145,129): #ffffff, the reference has #f6f6f6) |
| font-stretch-12.html | limit (font-stretch) | the font-stretch property is not in the style engine: FontQuery has no stretch, so only normal (100%) is ever asked for; the @font-face font-stretch descriptor is read and ranked (fonts::face::rank_faces) but cannot be selected. Later step (8878 pixels differ (largest difference 255); first at (145,129): #ffffff, the reference has #f6f6f6) |
| font-stretch-14.html | limit (font-stretch) | the font-stretch property is not in the style engine: FontQuery has no stretch, so only normal (100%) is ever asked for; the @font-face font-stretch descriptor is read and ranked (fonts::face::rank_faces) but cannot be selected. Later step (8878 pixels differ (largest difference 255); first at (145,129): #ffffff, the reference has #f6f6f6) |
| font-stretch-15.html | limit (font-stretch) | the font-stretch property is not in the style engine: FontQuery has no stretch, so only normal (100%) is ever asked for; the @font-face font-stretch descriptor is read and ranked (fonts::face::rank_faces) but cannot be selected. Later step (8878 pixels differ (largest difference 255); first at (145,129): #ffffff, the reference has #f6f6f6) |
| matching/font-weight-search-direction.html | limit (system font weight matching) | the second half of the test uses the installed Times New Roman: DirectWrite's GetFirstMatchingFont gives regular for 501 (the nearest weight), CSS 5.2 wants bold; the @font-face half uses rank_faces and is right (3122 pixels differ (largest difference 255); first at (23,370): #f2f2f2, the reference has #ffffff) |
| rex-in-monospace.html | limit (rex unit) | the rex unit (css-values-4) is not implemented (890 pixels differ (largest difference 255); first at (7,71): #ffffff, the reference has #f1f1f1) |
| ric-in-monospace.html | limit (ric unit) | the ric unit (css-values-4) is not implemented (807 pixels differ (largest difference 255); first at (8,59): #ffffff, the reference has #555555) |
| separators.html | bug (U+2028/U+2029 not drawn as spaces) | not investigated in 2d-1: line and paragraph separators in a web font differ from the reference; not a font loading problem (the other three paragraphs match) (548 pixels differ (largest difference 170); first at (27,296): #cccccc, the reference has #ffffff) |
| standard-font-family-6.html | limit (default font) | the text must fall back to Times; with no usable family the platform's default font is Segoe UI (decisions.md 1b-3) (27852 pixels differ (largest difference 255); first at (129,59): #ffffff, the reference has #dedede) |

## 通过

- downloadable-font-in-iframe-print.html
- downloadable-font-print.html
- first-available-font-001.html
- first-available-font-002.html
- first-available-font-003.html
- first-available-font-004.html
- first-available-font-005.html
- first-available-font-006.html
- first-available-font-007.html
- font-face-sign-function.html
- font-face-stretch-auto-static.html
- font-face-stretch-auto-variable.html
- font-face-stretch-default-variable.html
- font-face-weight-auto-static.html
- font-face-weight-auto-variable.html
- font-face-weight-default-variable.html
- font-stretch-01.html
- font-stretch-06.html
- font-stretch-13.html
- font-stretch-16.html
- font-stretch-17.html
- font-stretch-18.html
- font-weight-normal-variable.html
- lang-attribute-affects-rendering-of-second-text-run.html
- lang-attribute-affects-rendering.html
- matching/font-unicode-presented-as-emoji-outline.html
- standard-font-family-14.html
- standard-font-family-15.html
- standard-font-family-16.html
- standard-font-family-20.html
- synthetic-bold-out-of-capabilities-range.html
- variations/font-weight-metrics.html
