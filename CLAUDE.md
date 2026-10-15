# qinglan

小、快、省内存的浏览器，内核用 Rust 自己写。总体计划见 `PLAN.md`。

## 先读

1. `PLAN.md`：定位、硬规矩、四层路线。要偏离计划，先停下来说明原因，等确认。
2. `docs/L1-TASK.md`：当前任务（第 1 层，按顺序 1a → 1b → 1c）。
3. 规范：`docs/spec/`（只放本地，不提交）。HTML、CSS、URL、编码的细节一律对照规范原文写，不凭记忆；代码里注明出处，比如 `// HTML 13.2.5.1`、`// CSS2 8.3.1`、`// css-text-3 4.1.3`。

## 硬规矩

`PLAN.md` 里的 6 条硬规矩优先于任何功能。新功能只要破坏其中一条就不加。

## 技术决定

- Rust 2024 edition，版本由 `rust-toolchain.toml` 锁定。
- 许可证：MIT + Apache-2.0 双许可。依赖只能用兼容的宽松许可证（MIT、Apache-2.0、BSD、Zlib）。
- 产品代码不用第三方库，要加先说明理由，等确认。只在测试里用的开发依赖可以用宽松许可证的库，在报告里列出来。
- `qinglan-engine` 是纯 Rust：不调用 Windows，`forbid(unsafe_code)`。字体、网络、图片、编码都通过 `platform` 接口拿。
- `unsafe` 只许出现在 `qinglan-win` 的 `src/sys/` 下（Windows 函数和 COM 接口的声明和调用），每处注明为什么安全。Windows 的声明自己写，不加 windows-sys / windows 库。
- 代码、注释、提交信息用英文；`docs/` 下的文档用中文。界面文字中英双语，跟系统语言走。
- Edge、Chrome 只当测试时的参考答案，不进产品。
- 不读、不碰用户自己浏览器里的资料（书签、Cookie、密码、历史记录）。

## 范围

- 一次只做一步。只做任务单要求的，不顺手加功能、加配置项、加抽象层。
- 任务单"不做"里的东西，遇到了按任务单写的退路处理（比如浮动当普通块），不提前做。

## 健壮性

- 所有网页内容（HTML、CSS、图片、响应头、网址）都当作不可信的输入：可能是坏的，也可能是故意构造的。
- 内核代码不许对来自网页的数据用 `unwrap()`/`expect()`、下标越界、panic（用 clippy 规则强制）。
- 凡是可能被放大的地方都设上限：嵌套深度、元素数、规则数、`var()` 展开长度、`@import` 层数、图片像素数、响应大小、排版工作量。超了就截断，或者给出明确提示；不能卡死，不能吃光内存。
- 不管网页多烂，结果只有两种：显示出来，或者明确的错误页。

## 完成的标准

- 跑测试，贴出真实输出，不接受"应该能通过"。报告里提到的每项检查，报告前都要重新跑一遍。
- 性能和内存在插电时量，用电池时的数字不算。
- 规范没写清、或者自己做了选择的地方，在 `docs/decisions.md` 里记下选了什么、为什么。
- 每一步收尾做一次独立审查（清单式），审查意见和处理结果一起报告。

## 工具

- Python：`C:\Users\Vincent\AppData\Local\Programs\Python\Python312\python.exe`（不一定在 PATH 里）。
- 不要启动用户的 Word、Office 等程序。
- 不要搜索整个磁盘（比如 `find /`），这种命令会在后台一直跑。Windows SDK 的头文件在 `C:\Program Files (x86)\Windows Kits\10\Include\<版本>\` 下。
- 量性能前先确认插着电（`powershell -NoProfile -Command "(Get-CimInstance Win32_Battery).BatteryStatus"` 输出 2），也没有别的重活在跑；冷启动要每次新开进程来量，不能用同一进程里热缓存的数字。
