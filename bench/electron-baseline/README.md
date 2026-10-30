# 最小 Electron 基线

用来和 MoeWallpaper 做**同口径**对比。这是 hello world 级别的最小应用，
不是 VS Code 或 Discord——拿完整产品当基线等于偷换概念，见
[`docs/BENCHMARK.md`](../../docs/BENCHMARK.md) §0。

## 跑法

```powershell
cd bench\electron-baseline
npm ci
npx electron-builder --win nsis
```

产物：`dist\BaselineElectron Setup 1.0.0.exe`，然后对**它**执行
`docs/BENCHMARK.md` §2.1–2.4 的同一套测量命令。

## 当前状态（2026-10-30）

**打包已成功**，尺寸已测：

```text
安装包     121,183,964 B = 115.57 MB
安装内容   374,097,173 B = 356.77 MB   （dist\win-unpacked，73 个文件）
```

这次一次过，用的网络是手机热点 + 代理。之前失败的两次是
`connect ETIMEDOUT 20.205.243.166:443` 和 `The server aborted pending request`，
说明工具链下载（electron 压缩包 + NSIS + 7zip）对**链路稳定性**很敏感，重试是有效手段。

**四个数字都测完了**（口径见 `docs/BENCHMARK.md` §2.3 / §2.4）：

```text
安装包     121,183,964 B = 115.57 MB
安装内容   374,097,173 B = 356.77 MB   （dist\win-unpacked，73 个文件）
冷启动     97–119 ms，中位数 104.5 ms（10 发，空白窗口）
空闲内存   133.5 MB 私有（3 次：133.5 / 135.4 / 133.2，静置 60 秒，4 个进程）
```

**必须先搬到项目目录之外再跑**，理由见下面那一节：

```powershell
robocopy dist\win-unpacked D:\bench-electron /MIR
powershell -ExecutionPolicy Bypass -File .\measure.ps1
```

## 坑：窗口会自己消失

`main.js` 最初写成 `new BrowserWindow(...)`，不保存返回值。Electron 里
`BrowserWindow` 的 JS 包装对象一旦被 V8 回收，**原生窗口会被自动关闭**，接着
`window-all-closed` 触发 `app.quit()`——应用在启动几十秒后安静地退出。

它的杀伤力在于**伪装成别的问题**：没有错误弹窗、没有崩溃事件、ExitCode 是 0。
第一次测内存时表现为"启动 60 秒后 `Get-Process` 查不到进程"，很容易被误判成
程序没起来或者机器有问题。

修法就是留一个模块级引用（现在的写法）。`measure.ps1` 因此把"进程是否还活着"
做成硬检查：测不到就报错，不返回数字。

## 坑：启动前弹「打开文件 - 安全警告」

现象：脚本跑 10 次，其中 9 次报"窗口没出现或进程提前退出"，屏幕上还留着一个
「打开文件 - 安全警告：无法验证发布者」的窗口。

**那个弹窗不是程序弹的**，程序当时还没被创建——是 Windows Shell 在 `Start-Process`
（ShellExecute）阶段做发布者验证时拦下来的。于是：

- 计时里混进了等人的那几秒（测出一发 14207 ms，纯粹是人的反应时间）；
- 进程可能在弹窗被取消后直接退出，看起来像"程序起不来"。

修法在 `measure.ps1` 里：用 `[System.Diagnostics.Process]::Start` 配
`UseShellExecute = $false`，走 CreateProcess，完全绕开 Shell。这不只是躲弹窗，也是
更干净的测量——Shell 扩展加载本来就和冷启动无关。

这个坑和 Electron 无关，**MoeWallpaper 自己也会中**，见
[`docs/BENCHMARK.md`](../../docs/BENCHMARK.md) §2.3。

## 坑：在项目目录里启动就崩（`0x80000003`，什么都不留）

症状：双击 exe → SmartScreen 拦一次 → 点"仍要运行" → **窗口不出来**。从 PowerShell
启动时进程在 50–100 ms 内退出，`ExitCode = -2147483645`（`0x80000003`，
`STATUS_BREAKPOINT`），**stderr 为空**，事件日志和 WER 里**没有任何记录**。

四条假设都被实验推翻：

| 假设 | 实验 | 结果 |
| --- | --- | --- |
| Shell 的「无法验证发布者」弹窗吃掉了计时 | 改用 `UseShellExecute = $false` 走 CreateProcess | 弹窗消失，**照旧崩** |
| 第三方杀软拦截（本机 Kaspersky 21.26 + McAfee 残留） | 暂停卡巴保护后重试 | **照旧崩** |
| `app.asar` 损坏 / 完整性校验失败 | 手工解析 asar（5 个文件哈希全对）；解析 PE 取 `.rsrc`，里面没有 `ElectronAsarIntegrity` 资源 | 不是它 |
| electron-builder 改坏了 exe | 拿同一个 zip 里的原始 `electron.exe` 对照：单独跑正常，指向本应用就崩 | 不是它 |

**真正的原因是产物所在的目录。** 同一份文件、同一个 exe：

```text
D:\el-probe3\...                          → 窗口正常出现
D:\meowallpaper\el-pristine\...           → 窗口正常出现
D:\meowallpaper\abcdefghij\…\el-probe\…   → 窗口正常出现
D:\meowallpaper\Meowallpaper\bench\…      → 崩（换个目录名就不崩，所以不是路径长度）
```

搬到 `D:\bench-electron` 之后一切正常，上面那四个数字就是这么测出来的。

**机制没有查明，只确认了边界**：在项目目录树内启动会崩，搬出去就正常。已经排除：
路径长度、工作目录、`ELECTRON_RUN_AS_NODE`、`NODE_COMPILE_CACHE`、组件完整性、杀软。

**排查纪律**：这类不留痕的闪崩没有单一因果。每排除一条都要写清"用什么实验排除的"，
否则下次还会从同一个假设重新开始——这一轮为此白跑了三次。
