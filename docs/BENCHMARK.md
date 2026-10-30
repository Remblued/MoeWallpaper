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

⚠️ **历史数据不一致**：2026-10-30 记的 MoeWallpaper 冷启动 41 ms 是用旧的
`Start-Process`（Shell 路径）测出来的。在按本节口径补测之前，两个基线的冷启动
**不能直接对比**——这条已记在 §5 待办里。

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

跑 `bench/electron-baseline/measure.ps1`（普通桌面会话，它会启动 GUI 程序）：

```powershell
cd bench\electron-baseline
powershell -ExecutionPolicy Bypass -File .\measure.ps1
```

脚本按 2.3 / 2.4 的口径测，并且在**窗口没出现**或**进程提前退出**时报错，而不是给出
一个数字。测不到就不许填，这一条比"把表填满"重要。

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
安装包         1.69 MB         115.57 MB
安装后         5.19 MB         356.77 MB
冷启动(中位)    41 ms           TBD
空闲内存        212 MB（私有）   TBD
```

**安装包** 1,773,651 字节。**安装后** 5,443,478 字节（`moe-wallpaper.exe`
5,238.5 KB + `uninstall.exe` 77.4 KB），不含 WebView2 运行时——
`tauri.conf.json` 没配 `webviewInstallMode`，走默认的 `downloadBootstrapper`，
即 WebView2 由安装器在目标机器上按需下载。

**冷启动 10 次原始值（ms）**：

```text
200, 54, 45, 37, 33, 46, 37, 29, 49, 33
min=29  median=41  max=200
```

首次 200 ms 含磁盘冷读，按要求**没有剔除**（§0 第 5 条：异常值不删，单独写出来）。

⚠️ 这 10 发是用**已废弃的** `Start-Process`（Shell 路径）测的，见 §2.3。在按新口径
补测之前，41 ms 只用于"本项目自身的历史对比"，**不用于和 Electron 基线对比**。

**空闲内存 3 次，启动后静置 60 秒，6 个进程**（应用 1 + WebView2 5）：

```text
工作集  374.0 / 371.2 / 364.8 MB   中位数 371.2 MB
私有    211.0 / 214.5 / 212.5 MB   中位数 212.5 MB
```

口径：**私有内存 212 MB 是主口径**，因为工作集直接相加会重复计算多个进程共享的
DLL。工作集 371 MB 作为参考。

状态说明：测量时**网格已加载 24 张 wallhaven 缩略图**（截图确认，界面显示"共 24 张图片"）。
这一点很关键——之前一次在断网状态测到 365 MB 工作集，和这次几乎一样，
说明**内存大头是 WebView2 外壳，不是图片内容**。换句话说，这个数字不会因为
图源换成更"重"的而大幅变化，但也没法通过优化图片加载来明显降低。

单进程明细（一次采样）：

```text
类型           工作集    私有
gpu-process    83.0    116.9
browser       129.0     39.2
renderer       69.3     28.3
utility        40.6     12.5
utility        20.9      7.7
browser        31.4      6.4
```

**注意 gpu-process 的私有内存 116.9 MB，是整个应用里最高的一项**，比主进程
（6.4 MB）高一个数量级。想真正降内存，方向在 WebView2 的 GPU 合成，不在我们的
Rust 代码。

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
（5.19 MB）的 46 倍。安装包整体差 68 倍（115.57 / 1.69）。

**冷启动和空闲内存这两格仍然是 `TBD`**，原因见 §3.2。

**为什么 2026-10-30 没测到（三个假设都被推翻）**

现象：进程启动后 50–100 ms 退出，`ExitCode = -2147483645`（`0x80000003`，
`STATUS_BREAKPOINT`）；stderr 为空；事件日志没有 `Application Error`；WER 没有报告。

已排除：

| 假设 | 怎么排除的 | 结论 |
| --- | --- | --- |
| Shell 发布者验证的弹窗吃掉了时间 | 改 `UseShellExecute = $false` 后不再弹「打开文件 - 安全警告」，但照旧崩 | 弹窗和崩溃是两件事 |
| 第三方杀软拦截 | 本机有 Kaspersky 21.26（`KLIF`/`klflt` 驱动）与 McAfee 残留；**暂停卡巴保护后仍然崩** | 不是它 |
| `app.asar` 损坏 / 完整性校验失败 | 手工解析 asar：7,360 B、结构正常、`main.js` 与仓库一致；PE `.rsrc` 里**没有** `ElectronAsarIntegrity` 资源，即不存在可失败的校验 | 不是它 |

一个有力的反证：**第一次打包（不同的 `main.js`）时窗口 10/10 正常出现**。所以不是
"这台机器跑不了 Electron"，而是"这一次打出来的产物有问题"。

结论：这两格保持 `TBD`，不接受估算。下一步按可能性排序：

1. 删掉 `dist` **干净重建**——排除增量打包产出的坏产物；
2. 仍不行就把 electron 降到上一个稳定大版本，排除新版 Electron 与 electron-builder
   的兼容问题；
3. 两条都不解决，就在 README 里如实写"本机无法测量"并保留 `TBD`。

### 待办

- [x] Electron 基线的**尺寸**（`bench/electron-baseline/`，已打包测量）
- [ ] Electron 基线的**冷启动 + 空闲内存**（`bench/electron-baseline/measure.ps1`）
- [ ] 用 §2.3 的新口径（CreateProcess）**复测 MoeWallpaper 冷启动**，替换 41 ms
- [ ] 网络恢复后复测一次，确认数字可复现

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
| 被测程序启动即退，退出码 `-2147483645`（`STATUS_BREAKPOINT`），stderr 为空、WER 也没有记录 | 这一类症状**没有单一因果**，必须逐条实验排除：① Shell 弹窗占用了计时（§2.3）；② 第三方杀软注入/拦截——**本例暂停卡巴后仍崩，已排除**；③ 打包产物本身损坏——先做干净重建。凭感觉定一个原因是最容易出错的做法 |
