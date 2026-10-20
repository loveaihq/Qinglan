# WPT `css/CSS2/bidi-text`

测试平台（每个字 1em 见方），800×600，`qinglan-cli wpt css/CSS2/bidi-text`，用时 7.5 秒。

- 通过 **69 / 103**（67.0%）
- bug 类失败：**34**
- 跳过：无

失败分类：bug (edges of a box split by bidi reordering) 34

## 失败

| 测试 | 类别 | 说明 |
|---|---|---|
| bidi-001.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (560 pixels differ (largest difference 255); first at (315,144): #ffffff, the reference has #0000ff) |
| bidi-002.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (560 pixels differ (largest difference 255); first at (342,193): #ffffff, the reference has #000080) |
| bidi-003.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (3672 pixels differ (largest difference 255); first at (230,168): #000080, the reference has #ffa500) |
| bidi-004.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (4040 pixels differ (largest difference 255); first at (438,264): #000080, the reference has #ffffcc) |
| bidi-005a.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (5506 pixels differ (largest difference 255); first at (96,210): #ff00ff, the reference has #000000) |
| bidi-005b.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (10240 pixels differ (largest difference 255); first at (128,216): #000000, the reference has #ff00ff) |
| bidi-006a.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (5506 pixels differ (largest difference 255); first at (96,210): #ff00ff, the reference has #000000) |
| bidi-006b.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (10240 pixels differ (largest difference 255); first at (128,216): #000000, the reference has #ff00ff) |
| bidi-007a.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (5506 pixels differ (largest difference 255); first at (96,210): #ff00ff, the reference has #000000) |
| bidi-007b.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (10240 pixels differ (largest difference 255); first at (128,216): #000000, the reference has #ff00ff) |
| bidi-008a.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (5506 pixels differ (largest difference 255); first at (96,210): #ff00ff, the reference has #000000) |
| bidi-008b.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (10240 pixels differ (largest difference 255); first at (128,216): #000000, the reference has #ff00ff) |
| bidi-009a.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (5506 pixels differ (largest difference 255); first at (96,210): #ff00ff, the reference has #000000) |
| bidi-009b.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (10240 pixels differ (largest difference 255); first at (128,216): #000000, the reference has #ff00ff) |
| bidi-010a.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (5506 pixels differ (largest difference 255); first at (96,210): #ff00ff, the reference has #000000) |
| bidi-010b.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (10240 pixels differ (largest difference 255); first at (128,216): #000000, the reference has #ff00ff) |
| bidi-011.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (9320 pixels differ (largest difference 255); first at (171,109): #ffa500, the reference has #ffffff) |
| bidi-box-model-001.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (2732 pixels differ (largest difference 255); first at (8,75): #ffe93f, the reference has #ffffff) |
| bidi-box-model-002.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (2732 pixels differ (largest difference 255); first at (8,75): #ffe93f, the reference has #ffffff) |
| bidi-box-model-011.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (1600 pixels differ (largest difference 255); first at (128,104): #ffa500, the reference has #000080) |
| bidi-box-model-014.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (1600 pixels differ (largest difference 128); first at (8,104): #000080, the reference has #008080) |
| bidi-box-model-015.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (1600 pixels differ (largest difference 128); first at (8,104): #000080, the reference has #008080) |
| bidi-box-model-020.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (1600 pixels differ (largest difference 255); first at (248,104): #000080, the reference has #ffa500) |
| bidi-box-model-023.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (5200 pixels differ (largest difference 255); first at (128,104): #ffa500, the reference has #008080) |
| bidi-box-model-024.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (5200 pixels differ (largest difference 255); first at (128,104): #ffa500, the reference has #008080) |
| bidi-box-model-029.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (1600 pixels differ (largest difference 128); first at (8,104): #000080, the reference has #008080) |
| bidi-box-model-030.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (1600 pixels differ (largest difference 128); first at (8,104): #000080, the reference has #008080) |
| bidi-box-model-032.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (1600 pixels differ (largest difference 255); first at (128,104): #ffa500, the reference has #000080) |
| bidi-box-model-038.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (5200 pixels differ (largest difference 255); first at (128,104): #ffa500, the reference has #008080) |
| bidi-box-model-039.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (5200 pixels differ (largest difference 255); first at (128,104): #ffa500, the reference has #008080) |
| bidi-box-model-041.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (1600 pixels differ (largest difference 255); first at (248,104): #000080, the reference has #ffa500) |
| bidi-breaking-002.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (1444 pixels differ (largest difference 255); first at (35,189): #ffffff, the reference has #0000ff) |
| bidi-breaking-003.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (28297 pixels differ (largest difference 255); first at (254,181): #c0c0c0, the reference has #ffffff) |
| unicode-bidi-002.xht | bug (edges of a box split by bidi reordering) | an inline box that reordering cuts into pieces gets its start and end edge as separate units placed by their bidi level; browsers glue the start edge to the left (right for rtl) of the first piece and the end edge to the right of the last piece; not fixed in 2d-2a, see decisions.md 2d-2a (1536 pixels differ (largest difference 255); first at (566,66): #000000, the reference has #ffffff) |

## 通过

- bidi-box-model-003.xht
- bidi-box-model-004.xht
- bidi-box-model-005.xht
- bidi-box-model-006.xht
- bidi-box-model-007.xht
- bidi-box-model-008.xht
- bidi-box-model-009.xht
- bidi-box-model-010.xht
- bidi-box-model-012.xht
- bidi-box-model-013.xht
- bidi-box-model-016.xht
- bidi-box-model-017.xht
- bidi-box-model-018.xht
- bidi-box-model-019.xht
- bidi-box-model-021.xht
- bidi-box-model-022.xht
- bidi-box-model-025.xht
- bidi-box-model-026.xht
- bidi-box-model-027.xht
- bidi-box-model-028.xht
- bidi-box-model-031.xht
- bidi-box-model-033.xht
- bidi-box-model-034.xht
- bidi-box-model-035.xht
- bidi-box-model-036.xht
- bidi-box-model-037.xht
- bidi-box-model-040.xht
- bidi-box-model-042.xht
- bidi-box-model-043.xht
- bidi-box-model-044.xht
- bidi-box-model-045.xht
- bidi-breaking-001.xht
- bidi-glyph-mirroring-001.xht
- bidi-glyph-mirroring-002.xht
- bidi-inline-001.xht
- bidi-inline-002.xht
- direction-001.xht
- direction-002.xht
- direction-003.xht
- direction-applies-to-001.xht
- direction-applies-to-002.xht
- direction-applies-to-003.xht
- direction-applies-to-004.xht
- direction-applies-to-006.xht
- direction-applies-to-007.xht
- direction-applies-to-008.xht
- direction-applies-to-009.xht
- direction-applies-to-012.xht
- direction-applies-to-013.xht
- direction-applies-to-014.xht
- direction-applies-to-015.xht
- line-breaking-bidi-001.xht
- line-breaking-bidi-002.xht
- line-breaking-bidi-003.xht
- unicode-bidi-001.xht
- unicode-bidi-003.xht
- unicode-bidi-004.xht
- unicode-bidi-applies-to-001.xht
- unicode-bidi-applies-to-002.xht
- unicode-bidi-applies-to-003.xht
- unicode-bidi-applies-to-004.xht
- unicode-bidi-applies-to-006.xht
- unicode-bidi-applies-to-007.xht
- unicode-bidi-applies-to-008.xht
- unicode-bidi-applies-to-009.xht
- unicode-bidi-applies-to-012.xht
- unicode-bidi-applies-to-013.xht
- unicode-bidi-applies-to-014.xht
- unicode-bidi-applies-to-015.xht
