# 体积与性能测量方法

README 里的"轻量"是可复现的声明，不是形容词。**任何写进 README 的数字都必须来自这里
记录的流程**；没测过的格子写 `TBD`，不允许估、不允许抄别人的数据。

## 0. 四条原则

1. 先测量，后写数字。
2. 数字与宣称冲突时，改宣称。
3. 每组数字都要带上机器信息和日期（第 5 节）。
4. 基线用**最小 Electron 应用**，不用 VS Code / Discord——功能完全不同，拿来比就是
   偷换概念。

## 1. 测量前先固定条件

- 同一台机器、同一天、同一构建配置（一律 `release`）。
- 插电、高性能电源模式，关掉浏览器和构建任务。
- **记录全部安全软件，不是只记 Defender。** 更常见的情况是机器装着第三方杀软，此时
  Defender 的实时保护会被自动关掉——只写"Defender 实时保护已关闭"会让读者以为这台
  机器没有防护在干扰，实际恰恰相反。

  ```powershell
  Get-CimInstance -Namespace root/SecurityCenter2 -ClassName AntiVirusProduct |
    Select-Object displayName, productState
  Get-CimInstance Win32_SystemDriver |
    Where-Object { $_.State -eq 'Running' -and $_.Name -match 'klif|klflt|mfe|mca' } |
    Select-Object Name, State
  ```

  内核过滤驱动（例如卡巴的 `KLIF` / `klflt`）会 hook **每一次进程创建**，直接影响冷启动
  测量，还可能让未签名的被测程序根本起不来（症状见 §6）。
- **两个基线必须在同一个防护状态下测。** 一边暂停防护、一边不暂停，比出来的数字没有
  意义。
- 每组至少 10 次，报**中位数**；异常值不删，单独写出来。
- 记录 `git rev-parse --short HEAD` 的 commit hash。

## 2. 四个指标

### 2.1 安装包体积

```powershell
Get-ChildItem src-tauri\target\release\bundle\nsis\*.exe |
  Select-Object Name, @{ n = 'MB'; e = { [math]::Round($_.Length / 1MB, 2) } }
```

这是用户真正下载的东西，也是 README 里唯一可以对外说的"安装包大小"。

### 2.2 安装后磁盘占用

装到一个空目录（例如 `D:\bench\install`），然后：

```powershell
$sum = (Get-ChildItem 'D:\bench\install' -Recurse -File | Measure-Object Length -Sum).Sum
'{0:N1} MB' -f ($sum / 1MB)
```

注意：不含 WebView2 运行时本身。Windows 11 自带，但旧系统上首次安装会下载它——记录里
要写明这一点，否则这个数字会被误读。

### 2.3 冷启动到窗口可见

**必须用 CreateProcess 启动，不要用 `Start-Process`。**

`Start-Process` 走的是 ShellExecuteEx，Windows Shell 会在**进程被创建之前**做一次
"发布者验证"。遇到未签名、或带"来自 Internet"标记（Zone.Identifier）的程序，它会弹
「打开文件 - 安全警告：无法验证发布者」。那个弹窗不是程序弹的——程序根本还没启动；
用户点一下，**人的反应时间就被算进了冷启动**。更麻烦的是这个污染长得像一个"很慢的
程序"，而不是像一个缺陷。Shell 扩展加载本身也属于与启动无关的开销。

```powershell
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName      = 'C:\Program Files\MoeWallpaper\moe-wallpaper.exe'
$psi.UseShellExecute = $false      # ← 关键：走 CreateProcess，绕开 Shell
$t = foreach ($i in 1..10) {
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $p  = [System.Diagnostics.Process]::Start($psi)
  while ($p.MainWindowHandle -eq 0) { Start-Sleep -Milliseconds 10; $p.Refresh() }
  $sw.Stop()
  $p.CloseMainWindow() | Out-Null
  $p.WaitForExit()
  $sw.ElapsedMilliseconds
}
$s = $t | Sort-Object
"min={0}ms  median={1}ms  max={2}ms" -f $s[0], ($s[4] + $s[5]) / 2, $s[9]
```

口径说明：这是"到窗口句柄出现"的时间，**不是**"首屏渲染完成"。WebView2 的渲染发生
在窗口之后，两者相差可能几百毫秒。README 必须按这个口径措辞。

✅ 更早记的 41 ms 是用旧的 `Start-Process`（Shell 路径）测的，**已作废**。按本节口径
复测得到 **17.5 ms**：同一台机器、同一个 exe，只换启动路径就差 23 ms 以上。

**轮询间隔也是口径的一部分。** 上面示例每 10 ms 轮询一次，所以量出来的下限就在 10 ms
量级；2026-10-31 有一批用了 5 ms，得到的中位数比同一 exe 的 10 ms 批次低约 2 ms。
换脚本时要跟着换轮询间隔，并且不同间隔的批次**不要混着平均**。

**刚装完的第一发要单列。** 新写盘的 exe 首次执行要过杀软扫描和冷读，2026-10-31 实测
第一发 203 ms、常态中位 31 ms。按 §0 第 5 条不剔除，写出来，中位数只用常态那批。

### 2.4 空闲内存

```powershell
$app = Get-Process -Name 'moe-wallpaper' -ErrorAction SilentlyContinue
$wv  = Get-Process -Name 'msedgewebview2' -ErrorAction SilentlyContinue |
       Where-Object { $_.CommandLine -like '*moewallpaper*' }
$sum = ($app.WorkingSet64 | Measure-Object -Sum).Sum +
       ($wv.WorkingSet64  | Measure-Object -Sum).Sum
'{0:N0} MB' -f ($sum / 1MB)
```

在启动后静置 **60 秒**再测（启动瞬间的峰值没有意义）。

坑：WebView2 是多进程的（browser / renderer / GPU / utility）。只报主进程会得到小得
离谱的数字，那是造假，不是优化。

两条补充（2026-10-31 加，都是实测踩出来的）：

- **按进程名筛不可靠。** 这台机器上同时跑着 6 个不属于本项目的 `msedgewebview2.exe`
  （Windows 搜索宿主）。上面的命令行筛选 `*moewallpaper*` 是对的，但更硬的办法是从
  app 进程按 `ParentProcessId` 递归拿整棵树——顺便还能拿到每个进程的角色。
- **只测"有图"一种状态会得出错误结论。** 图片解码的账主要落在私有内存上。同一版
  构建：无图 198.6 MB / 有图 465.8 MB（私有，差 267 MB）。要对齐 Electron 那种空白
  窗口基线，就必须另测一次无图状态；做法见 §5（把 `HTTP(S)_PROXY` 指向死端口，只挡
  被测进程，不动系统代理）。**报内存数字必须带上"界面是什么状态"。**

## 3. Electron 基线

已建在 `bench/electron-baseline/`，源码提交进仓库（`node_modules/`、`dist/` 由根
`.gitignore` 排除）。刻意做到**最小**：一个空白窗口，不加载任何图片、不做任何业务。
它代表 Electron 的下限，不是它做壁纸应用时的真实成本。

```powershell
cd bench\electron-baseline
npm install
npx electron-builder --win nsis
```

产物：`dist\BaselineElectron Setup 1.0.0.exe`（安装包）、`dist\win-unpacked\`（安装内容）。

### 3.1 尺寸怎么量

安装包就是 2.1 那条命令的产物。**安装后体积这一项没有"真装一遍"**：
electron-builder 的 NSIS 安装器就是把这 73 个文件原样铺到安装目录，所以量
`dist\win-unpacked\` 等同于量装完的目录，差别只有约 0.3 MB 的卸载器。这一点必须
写进记录，因为方法和 MoeWallpaper 那边（真装到 `D:\bench\install`）不完全一样。

```powershell
(Get-ChildItem 'bench\electron-baseline\dist\win-unpacked' -Recurse -File |
  Measure-Object Length -Sum).Sum
```

### 3.2 冷启动和空闲内存

**先把产物搬到项目目录之外再测。** 在项目目录树内启动这个 Electron 产物会崩
（`0x80000003`，机制未查明，见 §5），搬出去就正常：

```powershell
robocopy bench\electron-baseline\dist\win-unpacked D:\bench-electron /MIR
powershell -ExecutionPolicy Bypass -File bench\electron-baseline\measure.ps1
```

（`measure.ps1` 里写的是 `D:\bench-electron\BaselineElectron.exe`；换路径要一起改。）

脚本按 2.3 / 2.4 的口径测，并且在**窗口没出现**或**进程提前退出**时报错，而不是给出
一个数字。测不到就不许填，这一条比"把表填满"重要。

本项目的对照数字用同样的方法、同一时刻测：见 §5。

## 4. 对比纪律

- 只比同一指标、同一口径。
- 不对"最小 Electron 应用"以外的对象下结论。
- 如果 MoeWallpaper 某个指标反而不如基线，写出来，不要删。

## 5. 记录

### 第一次测量 · 2026-10-30

```text
日期        2026-10-30
commit      74d4170
机器        Intel Core i7-14650HX（24 逻辑核）/ 31.6 GB / Windows 11 家庭版 26200.9711
电源        未固定（读不到活动电源方案，如实记录）
Defender    实时保护关闭（RealTimeProtectionEnabled = False）
安全软件    Kaspersky 21.26（服务 AVP21.26；内核驱动 KLIF / klflt 运行中）
            + McAfee（服务已停止，注册残留）
            ↑ 事后补记：原记录只写了 Defender，漏了第三方杀软
构建        npm run tauri build（release, lto=true, strip=true, opt-level=3）
安装目录    D:\bench\install

               MoeWallpaper    Electron 基线
安装包         1.81 MB         115.57 MB
安装后         5.38 MB         356.77 MB
冷启动(中位)    29.5 ms         104.5 ms
空闲内存        232.9 MB（私有） 133.5 MB（私有）
```

> 四格现在都来自**同一版构建**（`a65d7ed`：加完详情页、搜索、翻页、分辨率筛选、主题）。
> 更早一版（`74d4170`）的冷启动中位数是 17.5 ms，见本节末尾「功能加完后的复测」。
> 两个数都真实，但**不能混着用**。

**安装包** 1,773,651 字节。**安装后** 5,443,478 字节（`moe-wallpaper.exe`
5,238.5 KB + `uninstall.exe` 77.4 KB），不含 WebView2 运行时——
`tauri.conf.json` 没配 `webviewInstallMode`，走默认的 `downloadBootstrapper`，
即 WebView2 由安装器在目标机器上按需下载。

**冷启动 10 次原始值（ms）**，两列都是同一台机器、同一防护状态、§2.3 的 CreateProcess 口径：

```text
MoeWallpaper   31, 17, 17, 22, 17, 18, 36, 17, 22, 17      min=17   median=17.5  max=36
Electron       100, 106, 110, 101, 103, 97, 102, 114, 108, 119   min=97  median=104.5 max=119
```

首发不是最慢的一发（缓存已热），所以这里没有需要单独说明的首发异常值。

⚠️ **更早记录的 41 ms 已作废**（原始值 200, 54, 45, 37, 33, 46, 37, 29, 49, 33）。
那 10 发是用 Shell 路径（`Start-Process`）测的，见 §2.3。同一个 exe、同一台机器，
只换启动路径就差 23 ms——§2.3 那条规矩不是形式主义。

**空闲内存 3 次，启动后静置 60 秒**：

```text
             进程数                     私有（中位数）                工作集（中位数）
MoeWallpaper 7（应用 1 + WebView2 6）    218.4 MB（213.9/218.4/222.0）  381.9 MB
Electron     4                          133.5 MB（133.5/135.4/133.2）  275.9 MB
```

**这一项本项目输了，输 85 MB。** 按 §4 第三条写出来，不删、不淡化。

单进程明细（各取一次采样）：

```text
MoeWallpaper
  应用       私有   6.3 MB   工作集  28.8 MB
  WebView2   私有 117.3 MB   工作集  81.3 MB   ← 和之前记的 GPU 进程 116.9 MB 一致
  WebView2   私有  37.2 / 28.7 / 12.6 / 8.7 / 3.1 MB

Electron 基线（类型未逐个标注）
  带窗口的   私有  28.8 MB   工作集  84.3 MB
  其余三个   私有  71.9 / 21.1 / 11.3 MB
```

口径：**私有内存是主口径**，工作集直接相加会重复计算多个进程共享的 DLL。
两边差别集中在"多出来的 WebView2 助手进程"和 GPU 进程的占用上；**没有做逐进程归因，
所以不写原因**——只记录事实。

状态说明：MoeWallpaper 测量时**网格已加载 24 张 wallhaven 缩略图**（界面显示"共 24 张
图片"）。之前一次断网状态下测到 365 MB 工作集，和这次几乎一样——说明**内存大头是
WebView2 外壳，不是图片内容**：既不会因为换更重的图源而大幅变化，也没法靠优化图片
加载明显降低。

**gpu 进程的私有内存（116.9 MB）是整个应用里最高的一项**，比主进程（6.4 MB）高一个
数量级。想降内存，方向在 WebView2 的 GPU 合成，不在我们的 Rust 代码。

### Electron 基线 · 同一天 · 同一台机器

工具链 electron-builder 26.18.0 / electron 45.1.0 / 目标 NSIS / 未做代码签名。
基线页面是**一个空白窗口**，不加载图片——下面每个数字都是 Electron 的下限。

```text
安装包     121,183,964 B = 115.57 MB   （dist\BaselineElectron Setup 1.0.0.exe）
安装内容   374,097,173 B = 356.77 MB   （dist\win-unpacked，73 个文件）
```

体积去哪了（`dist\win-unpacked` 内，按大小排序）：

```text
BaselineElectron.exe      238.23 MB   ← 单个可执行文件
dxcompiler.dll             24.58 MB
resources.pak              12.75 MB
icudtl.dat                 10.32 MB
LICENSES.chromium.html      5.44 MB
vk_swiftshader.dll          5.20 MB
d3dcompiler_47.dll          4.53 MB
ffmpeg.dll                  2.95 MB
locales\*.pak              48.82 MB   （55 个语言包，本项目一个都用不上）
```

对照：`BaselineElectron.exe` 一个文件（238.23 MB）就是 MoeWallpaper 整个安装目录
（5.38 MB）的 44 倍。安装包整体差 64 倍（115.57 / 1.81）。

**这个基线一开始根本跑不起来，原因是一类很容易被误判的问题，必须记下来。**

现象：进程启动后 50–100 ms 退出，`ExitCode = -2147483645`（`0x80000003`，
`STATUS_BREAKPOINT`）；stderr 为空；事件日志没有 `Application Error`；WER 没有报告。

四条假设全部被实验推翻：

| 假设 | 怎么排除的 | 结论 |
| --- | --- | --- |
| Shell 发布者验证的弹窗吃掉了时间 | 改 `UseShellExecute = $false` 走 CreateProcess，弹窗消失 | 弹窗和崩溃是两件事 |
| 第三方杀软拦截 | 本机 Kaspersky 21.26（`KLIF`/`klflt` 驱动）；暂停保护后仍然崩 | 不是它 |
| `app.asar` 损坏 / 完整性校验失败 | 手工解析 asar（7,360 B，`main.js` 与仓库一致、5 个文件哈希全对）；PE `.rsrc` 里没有 `ElectronAsarIntegrity` 资源 | 不是它 |
| electron-builder 改坏了 exe | 用同一个 zip 里的原始 `electron.exe` 做对照：单独跑正常，指向我们的应用就崩 | 不是它 |

**真正的原因：产物所在的目录。** 同一份文件、同一个 exe、同样的环境变量：

```text
D:\el-probe3\...                          → 窗口正常出现
D:\meowallpaper\el-pristine\...           → 窗口正常出现
D:\meowallpaper\abcdefghij\…\el-probe\…   → 窗口正常出现
D:\meowallpaper\Meowallpaper\bench\…      → 崩（换个目录名就不崩，所以不是路径长度）
```

把 `win-unpacked` 整个搬到 `D:\bench-electron` 之后，**真正的
`BaselineElectron.exe` 正常启动**，上面那两组数字就是这么测出来的。

**机制没有查明**，只确认了边界：**在项目目录树内启动崩溃，搬出去就正常**。已知被
排除的：路径长度、工作目录、`ELECTRON_RUN_AS_NODE`、`NODE_COMPILE_CACHE`、组件
完整性、杀软。要不要继续查是另一个问题——**测量不该被这件事挡住，所以换目录测，并把
边界写清楚**。

### 重新构建后的体积（2026-10-30 晚，`a65d7ed`）

加完详情页、搜索、翻页、分辨率筛选、主题之后重新打包。这一节只讲体积；
冷启动和内存见下一节，两者是同一版构建测出来的。

```text
commit      a65d7ed
安装包     1,898,185 B = 1.81 MB   （MoeWallpaper_0.1.0_x64-setup.exe）
安装后     5,642,646 B = 5.38 MB   （moe-wallpaper.exe 5,563,392 + uninstall.exe 79,254）
```

和上一版（`74d4170`：1,773,651 / 5,443,478）相比，安装包 **+124,534 B（约 +7%）**，
安装后 +199,168 B。增长全部来自前端：多了详情页、搜索框、翻页栏、分辨率下拉和主题
切换，JS 从约 5 KB 涨到约 7.5 KB，样式从约 3.8 KB 涨到约 6.9 KB（gzip 前）。

### 功能加完后的复测（`a65d7ed`，同一台机器、同一套方法）

体积之后把时序也补测了，用的是**同一个脚本、同一防护状态**（Kaspersky 服务运行中、
Defender 实时关闭），所以这两组可以直接和上一版比。

```text
冷启动 10 发原始值（ms）
  37, 29, 30, 33, 17, 21, 21, 17, 32, 33     min=17  median=29.5  max=37

空闲内存 3 次（静置 60 秒，7 个进程 = 应用 1 + WebView2 6）
  私有   241.5 / 230.5 / 232.9 MB   中位数 232.9 MB
  工作集 406.3 / 397.7 / 398.6 MB   中位数 398.6 MB
```

对照加功能之前（`74d4170`）：冷启动 17.5 ms、内存 218.4 MB。

**两项都变差了：冷启动 +12 ms，内存 +14.5 MB。** README 一律采用新的、更保守的数字，
不用旧的。

需要说清楚的是：两次测量的中位数差了 12 ms，而每一组内部的极差（17–37）也很宽。
**差异来源没有查明**——可能是前端变大（多了详情页、搜索、翻页、分辨率、主题），也可能
只是这台机器当时跑的进程不一样。既然查不清，就按"较慢的那个"写，而不是挑好看的。

memory 单进程明细（一次采样）：

```text
应用          私有   6.2 MB   工作集  28.8 MB
WebView2      私有 127.0 MB   工作集  84.9 MB   ← GPU 进程
WebView2      私有  42.7 / 32.5 / 12.7 / 7.7 / 3.1 MB
```

GPU 进程从 117 MB 涨到 127 MB。**没有逐进程归因，只记录事实。**
**7% 的体积换四个功能，这个交换站得住。** 但它意味着 README 里所有体积数字都必须
跟着这次重测改，旧数字不能留——旧构建已经不存在了。

### v0.1.2 release · 2026-10-31（README 现在用的就是这一组）

```text
日期        2026-10-31（11:40–12:25）
commit      24f4cbc（v0.1.2；测量期间工作树干净，没有为测量改过代码）
机器        Intel Core i7-14650HX（24 逻辑核）/ 31.6 GB / Windows 11 家庭版 26200
电源        平衡（powercfg /getactivescheme = 381b4222-f694-41f0-9685-ff5bb260df2e）
            ← 与上一版记录不同：那次写"读不到活动电源方案"，这次读到了，但仍是平衡档
Defender    实时保护关闭（RealTimeProtectionEnabled = False）
安全软件    Kaspersky 21.26（内核驱动 KLIF.K4W-21-26 / klflt.K4W-21-26 运行中）
            + McAfee（服务已停止，注册残留）
后台负载    测量期间 DSH Desktop（Electron 外壳）和 Windows 搜索宿主在跑
构建        release（lto = true / strip = true / opt-level = 3）
安装        NSIS 静默安装 `/S /D=D:\bench\install`，装前先用旧版卸载器清干净
安装包      MoeWallpaper_0.1.2_x64-setup.exe
            sha256 03F24BE07FA3A6BEA3326804C45D3C30A68B4931F81956CA6391491C0AA638E4

               MoeWallpaper 0.1.2
安装包         1,903,435 B = 1.82 MB
安装后         5,656,982 B = 5.39 MB
冷启动(中位)    31 ms（10 发，10 ms 轮询；刚装完的第一发 203 ms 单独列出）
空闲内存        有图 465.8 MB 私有（7 发，187.0–580.5）
                无图 198.6 MB 私有（3 发，198.0–211.1）← 这一格才能和 Electron 的空白窗口比
```

**体积。** 安装包里只有 `moe-wallpaper.exe` 5,577,728 B 和 `uninstall.exe` 79,254 B，
合计 5,656,982 B；卸载器写成 `EstimatedSize = 5524 KB`。与上一版（`a65d7ed`：
1,898,185 / 5,642,646）相比，安装包 +5,250 B、安装后 +14,336 B——两次 bump 版本号、
加 safebooru 图源、加内容标注、玻璃主题改跟随系统，加起来只动了 5 KB 和 14 KB。
`asset` 全部编进 exe（前端产物由 `tauri-build` 内嵌，图源 JSON 用 `include_str!`），
所以体积增长只会出现在那 5.3 MB 的 exe 里。

**冷启动 20 发原始值（ms），两批分开报，不混口径：**

```text
第 1 批  刚装完第一次跑，5 ms 轮询，未重装：
  203, 33, 49, 31, 20, 33, 41, 46, 32, 32      min=20  median=33  max=203
第 2 批  同一个已安装 exe，10 ms 轮询（§2.3 示例的口径）：
  43, 28, 31, 31, 31, 30, 44, 31, 30, 31       min=28  median=31  max=44
```

第 1 批的 **203 ms 是"刚装完的第一发"**：新写盘的 exe 首次执行要过杀软扫描和冷读，
§6 里那条"首次启动特别慢"就是它。按 §0 第 5 条**不剔除**，单列出来；第 2 批是同一台
机器、同一个 exe、防护状态没变的常态，README 用第 2 批的 **31 ms**。

第 1 批用 5 ms 轮询、第 2 批用 10 ms 轮询，所以两批的中位数**不能相加再平均**——
轮询间隔本身就是这个口径的一部分（10 ms 轮询的下限是 10 ms 级）。

**空闲内存：这次测了"有图"和"无图"两种状态，因为只测一种会得出错误结论。**

方法：进程**树**（从 app 进程按 `ParentProcessId` 递归），不是按进程名或命令行筛。
原因很实在——这台机器上同时跑着 6 个**不属于本项目**的 `msedgewebview2.exe`
（`--webview-exe-name=SearchHost.exe`，Windows 搜索宿主）。按进程名统计会把它们算进来。
每次都是启动后静置 60 秒。

```text
有图（默认图源 360，界面"共 20 张图片"，7 次）
  私有    402.0 / 465.8 / 580.5 / 528.2 / 187.0 / 424.6 / 531.9   中位 465.8 MB
  工作集  450.9 / 413.7 / 487.2 / 459.2 / 532.8 / 430.2 / 437.3   中位 450.9 MB

无图（HTTP(S)_PROXY 指向死端口，fetch_images 必然失败，界面是错误态，3 次）
  私有    198.0 / 198.6 / 211.1                                    中位 198.6 MB
  工作集  371.6 / 371.6 / 373.4                                    中位 371.6 MB
```

无图状态是怎么做出来的：`net.rs` 只在 `ProxyEnable = 1` 时才给 reqwest 显式配代理，
本机系统代理是关着的，于是 reqwest 退回读 `HTTP_PROXY` / `HTTPS_PROXY` 环境变量。
把这两个变量指向 `127.0.0.1:1`（没人监听）就只挡住被测进程的取图请求，**不动系统代理、
不影响别的程序、不用改代码**。脚本：`bench/measure/measure-offline.ps1`。界面证据：
`PrintWindow` 截下的错误态写的是
`加载失败：请求 http://wallpaper.apc.360.cn/... error sending request for url (...)`。

**这次用的脚本都在 `bench/measure/`**（原样复制，只改了 CLI 启动器里一行路径；用法和
每个脚本对应哪个数字，见 `bench/measure/README.md`）。谁改了脚本，要同步改本节。

**为什么必须分两种状态**：§5 上一版记的 232.9 MB 是拿 wallhaven 缩略图测的，而 360 是
现在的默认图源。两个数差 2 倍以上，不去区分就会写成"内存涨了"或"内存降了"——两种说法
都是错的。

**有图那一栏为什么抖动这么大（187–580 MB），以及原因在哪：**

单进程明细（4 次采样，私有 / 工作集，MB）：

```text
角色                          #1                #2                #3                #4
gpu-process          426.9 / 127.8      82.2 /  78.4     324.8 / 113.7     431.0 / 123.9
browser               39.6 / 129.9      39.4 / 127.0      39.0 / 126.9      39.2 / 127.3
renderer              32.1 /  96.0      35.9 / 224.0      31.5 /  86.5      32.4 /  83.1
utility:NetworkService 12.8 / 40.9      12.8 /  41.0      12.7 /  40.7      12.6 /  40.7
utility:StorageService  7.7 / 20.9       7.7 /  20.9       7.7 /  20.9       7.7 /  20.9
app                     6.0 / 30.2       5.9 /  28.1       5.8 /  28.1       5.8 /  28.0
crashpad-handler        3.1 / 13.5       3.1 /  13.5       3.1 /  13.5       3.1 /  13.5
合计                  528.2 / 459.2     187.0 / 532.8     424.6 / 430.2     531.9 / 437.3
```

抖动几乎全在 **gpu-process 的私有内存**（82–480 MB）上；工作集反而稳（75–133 MB）。
第 2 次采样里 GPU 私有只有 82 MB，但**渲染进程工作集涨到 224 MB**——同一批图的内存
在"GPU 进程私有"和"渲染进程工作集"之间换了个记账位置，合计变化没那么大。
**为什么换位置没有查明**（可能是合成路径/光栅化的选择），所以只记录事实，不写原因。
这也是这次把采样从 3 发加到 7 发的原因：3 发里抽到 187 或者抽到 580 都会得到一个
看着很确定、其实没意义的数字。

**这 267 MB 的差值（465.8 → 198.6）是可解释的，不是玄学：360 的 `url_thumb` 就是原图。**

```text
实测 3 条（2026-10-31，同一个接口同一批数据）：
  2560×1440   309,650 B
  1920×1080   286,168 B
  1920×1080   363,464 B
```

`thumbnailUrl` 映射到 `url_thumb`，但这个字段返回的是全尺寸壁纸，不是缩略图。界面"共
20 张图片"= 浏览器要解码 20 张 1080p/1440p 图，按 1920×1080×4 B 算就是每张约 8 MB、
20 张约 166 MB，与上面 GPU 进程的几百 MB 私有内存在同一个量级。**所以本项目的内存大头
在这台机器上是"图片原图"，不是"WebView2 外壳"**——这句结论是对上一版记录的更正，
上一版写的是"内存大头是 WebView2 外壳，不是图片内容"，依据是断网测到 365 MB 工作集
和联网时"几乎一样"。那次比的是**工作集**，而外壳与图片的差别主要落在**私有内存**上：
无图 198.6 / 有图 465.8，差 267 MB。**旧说法作废，以本节为准。**

**和 Electron 基线怎么比（口径必须对齐）：**

```text
                    MoeWallpaper 0.1.2          Electron 基线
                    无图        有图             空白窗口
私有内存            198.6 MB    465.8 MB         133.5 MB
工作集              371.6 MB    450.9 MB         275.9 MB
进程数              7           7                4
```

Electron 基线是**一个不加载任何图片的空白窗口**，所以能和它对齐的只有我们"无图"
那一栏：**198.6 MB 对 133.5 MB，多 65.1 MB（1.49 倍）**；工作集 371.6 对 275.9，多
95.7 MB。多出来的部分在进程数上就能看出来：我们 7 个进程，Electron 4 个，空载时
`gpu-process` 一个就占 102.3 MB 私有。

"有图"那一栏（465.8 MB）**不能拿去和空白窗口的 133.5 MB 说"我们多 332 MB"**——那等于
拿"20 张 1080p 图"比"什么都不显示"。Electron 应用显示同样 20 张图也会涨，只是这个最小
基线没法测那一面。这条限制写在这里，README 也照这个口径措辞。

**和上一版记录的关系（不要混着引用）：**

```text
                     a65d7ed（2026-10-30）     24f4cbc（2026-10-31）
安装包               1,898,185 B = 1.81 MB     1,903,435 B = 1.82 MB
安装后               5,642,646 B = 5.38 MB     5,656,982 B = 5.39 MB
冷启动(中位)          29.5 ms                   31 ms
空闲内存(私有)        232.9 MB（有图，wallhaven） 465.8 MB（有图，360）
                                                198.6 MB（无图）
```

体积和冷启动只差一点点（安装包 +5,250 B、冷启动 +1.5 ms），可以当同一条线看。内存那一格
**不可比**：两次的界面状态不同（wallhaven 缩略图 vs 360 原图），而且这版之前从没测过
"无图"状态。上一版的 232.9 MB 保留在原处，不删、不改写。

### 本机测量踩坑（这次新增的）

- **`npm run tauri build` 在这个 harness 里会报 `unrecognized subcommand`。** 原因不是
  Tauri：这个环境里 `node` 实际是 DSH Desktop 的 Electron 可执行文件，
  `process.argv[0]` 是 `D:\apps\deepseek harness\DSH Desktop\DSH Desktop.exe`。
  `@tauri-apps/cli/tauri.js` 用它推断自己的二进制名，推断失败就把这个路径当成子命令
  塞到参数最前面。绕法：先改写 `process.argv` 再 require 它
  （`bench/measure/tauri-cli-workaround.js`，几行）。**这不是项目问题，是测量环境问题，
  别写进 README。**
- **截图会截到别人的窗口。** `CopyFromScreen` 抄的是屏幕，不是窗口；启动的窗口没有
  前台焦点时，截出来的是压在它上面的东西（第一次截到了旁边的 AI 客户端窗口，第二次
  截到一个浏览器页面）。能用的办法是 `PrintWindow(hwnd, hdc, PW_RENDERFULLCONTENT=2)`
  ——它按窗口渲染，被遮挡也能截到。`SetForegroundWindow` 会被 Windows 的前台锁拒绝
  （本次两次都失败，`GetForegroundWindow()` 返回的不是我们的窗口），不能指望它。
- **按进程名统计 WebView2 会把别人的进程算进来。** 本机有 6 个
  `msedgewebview2.exe` 属于 Windows 搜索宿主。§2.4 的脚本用命令行筛选
  （`*moewallpaper*`）是安全的，但按 `ParentProcessId` 从 app 递归更硬，且能顺带拿到
  每个进程的角色（browser / renderer / gpu-process / utility / crashpad）。

### 待办

- [x] Electron 基线的**尺寸**（`bench/electron-baseline/`）
- [x] Electron 基线的**冷启动 + 空闲内存**（搬到 `D:\bench-electron` 后测）
- [x] 用 §2.3 的新口径（CreateProcess）**复测 MoeWallpaper 冷启动**，41 ms 已作废
- [x] 功能加完后**复测冷启动和空闲内存**（29.5 ms / 232.9 MB，都变差了，README 已改）
- [x] 在 **v0.1.2 release** 构建上重测四个数字（31 ms / 1.82 MB / 5.39 MB / 465.8+198.6 MB）
- [x] 补齐**无图状态**的空闲内存，否则和 Electron 空白窗口没法对齐口径
- [ ] 查清 gpu-process 私有内存 82–480 MB 的抖动（换记账位置的那一下）。**这决定内存
      那一格能不能当单一数字对外说**
- [ ] 换一天复测一次，确认数字可复现
- [ ] （可选）查清"在项目目录树内启动即崩"的机制——不影响数字，但很想知道

### 记录模板

把测完的结果记在这里，README 的表格从这份数据抄过去。

```text
日期        2026-XX-XX
commit      XXXXXXX
机器        CPU / 内存 / Windows build
电源        插电 / 高性能
Defender    默认开启（+ 第三方杀软名字与驱动）
构建        npm run tauri build（release, lto=true, strip=true）

               MoeWallpaper    Electron 基线
安装包         ___ MB          ___ MB
安装后         ___ MB          ___ MB
冷启动(中位)   ___ ms          ___ ms
空闲内存       无图 ___ MB      ___ MB（空白窗口）
               有图 ___ MB      无对应格
               （两次都要带界面状态、采样次数和区间）
```

## 6. 已知陷阱

| 现象 | 原因 |
| --- | --- |
| 内存数字特别小 | 只算了主进程，漏了 WebView2 子进程 |
| 启动特别快 | 测的是热启动（进程还没完全退出就再启一次） |
| 首次启动特别慢 | Defender 首次扫描 / 磁盘冷读。**标注出来单独报告，不要静默剔除**（§0 第 5 条）；想要干净的中位数就测 11 次，把首发单独写 |
| 安装包特别大 | 用 dev 构建测的，或者没开 `[profile.release]` 的 lto/strip |
| 数字每次都不一样 | 没固定电源模式 / 没关后台任务；先固定条件再谈精度 |
| 测内存时进程不见了 | Electron 的 `BrowserWindow` 没被引用，被 GC 后窗口自动关闭，`window-all-closed` 让应用干净退出——**不报错、不留事件日志**，极容易被误判成"程序启动失败" |
| 冷启动忽快忽慢，慢起来像几秒到十几秒 | 启动命令走 Shell（`Start-Process`）时可能弹「打开文件 - 安全警告」，程序还没创建就在等人点鼠标。改用 `UseShellExecute = $false`（§2.3） |
| 某些指标每次都不一样，又说不出规律 | 先怀疑"测量工具污染被测对象"：脚本自己定期跑的东西（枚举进程、轮询磁盘）也会抢 CPU。把诊断性轮询降到 500 ms 级 |
| 内存差 2–3 倍，工作集却差不多 | **私有内存（commit）不是可加总的稳定量。** 同一构建、同一界面状态、7 次采样：私有 187.0–580.5 MB，工作集 413.7–532.8 MB。抖动集中在 `gpu-process` 的私有内存（82–480 MB），有一次它的私有只有 82 MB 而渲染进程工作集涨到 224 MB——内存只是换了个记账位置。**报中位数并写出区间，别只报一个数** |
| 截图里是别的窗口 / 桌面 | `CopyFromScreen` 抄的是**屏幕**，不是窗口；启动的窗口没有前台焦点时会截到压在它上面的东西。改用 `PrintWindow(hwnd, hdc, 2)`（`PW_RENDERFULLCONTENT`）。`SetForegroundWindow` 会被前台锁拒绝，别指望它 |
| `npm run tauri build` 报 `unrecognized subcommand 'D:\...\DSH Desktop.exe'` | 测量环境的坑，不是项目问题：这个 harness 里 `node` 实际是 DSH Desktop 的 Electron 可执行文件，`process.argv[0]` 是它自己，`@tauri-apps/cli/tauri.js` 据此推断二进制名失败，把该路径当成子命令塞进参数。先改写 `process.argv` 再 require 即可（见 §5 末） |
| 被测程序启动即退，退出码 `-2147483645`（`STATUS_BREAKPOINT`），stderr 为空、WER 也没有记录 | 这一类症状**没有单一因果**，必须逐条实验排除。本机实测到的一个原因：**产物放在项目目录树内**就会崩，搬到 `D:\bench-electron` 就正常（路径长度、工作目录、`ELECTRON_RUN_AS_NODE`、`NODE_COMPILE_CACHE`、组件完整性、杀软都已排除）。换目录再测，并把边界写进记录 |
