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

### 待办

- [x] Electron 基线的**尺寸**（`bench/electron-baseline/`）
- [x] Electron 基线的**冷启动 + 空闲内存**（搬到 `D:\bench-electron` 后测）
- [x] 用 §2.3 的新口径（CreateProcess）**复测 MoeWallpaper 冷启动**，41 ms 已作废
- [x] 功能加完后**复测冷启动和空闲内存**（29.5 ms / 232.9 MB，都变差了，README 已改）
- [ ] 换一天复测一次，确认数字可复现
- [ ] （可选）查清"在项目目录树内启动即崩"的机制——不影响数字，但很想知道

### 记录模板

把测完的结果记在这里，README 的表格从这份数据抄过去。

```text
日期        2026-XX-XX
commit      XXXXXXX
机器        CPU / 内存 / Windows build
电源        插电 / 高性能
Defender    默认开启
构建        npm run tauri build（release, lto=true, strip=true）

               MoeWallpaper    Electron 基线
安装包         ___ MB          ___ MB
安装后         ___ MB          ___ MB
冷启动(中位)   ___ ms          ___ ms
空闲内存       ___ MB          ___ MB
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
| 某个指标每次都不稳定，又说不出规律 | 先怀疑"测量工具污染被测对象"：脚本自己定期跑的东西（枚举进程、轮询磁盘）也会抢 CPU。把诊断性轮询降到 500 ms 级 |
| 被测程序启动即退，退出码 `-2147483645`（`STATUS_BREAKPOINT`），stderr 为空、WER 也没有记录 | 这一类症状**没有单一因果**，必须逐条实验排除。本机实测到的一个原因：**产物放在项目目录树内**就会崩，搬到 `D:\bench-electron` 就正常（路径长度、工作目录、`ELECTRON_RUN_AS_NODE`、`NODE_COMPILE_CACHE`、组件完整性、杀软都已排除）。换目录再测，并把边界写进记录 |
