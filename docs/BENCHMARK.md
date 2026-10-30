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
- Defender 实时保护保持默认，但要写进记录（它会显著影响冷启动）。
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

```powershell
$exe = 'C:\Program Files\MoeWallpaper\moe-wallpaper.exe'
$t = foreach ($i in 1..10) {
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $p  = Start-Process $exe -PassThru
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

放在 `bench/electron-baseline/`（**尚未创建**，做这一步时再建）：

```powershell
mkdir bench\electron-baseline
cd bench\electron-baseline
npm init -y
npm i -D electron electron-builder
# 写最小 main.js + index.html（一个空白窗口即可，不做任何业务）
npx electron-builder --win nsis
```

然后对产物执行 2.1–2.4 的**同一套命令**。基线源码要提交进仓库（可复现），但
`node_modules/`、`dist/`、`release/` 必须 ignore。

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
构建        npm run tauri build（release, lto=true, strip=true, opt-level=3）
安装目录    D:\bench\install

               MoeWallpaper    Electron 基线
安装包         1.69 MB         TBD
安装后         5.19 MB         TBD
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

### 待办

- [ ] Electron 基线（`bench/electron-baseline/`，依赖约 300 MB）
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
| 首次启动特别慢 | Defender 首次扫描；应标注"首次"并排除在 10 次之外 |
| 安装包特别大 | 用 dev 构建测的，或者没开 `[profile.release]` 的 lto/strip |
| 数字每次都不一样 | 没固定电源模式 / 没关后台任务；先固定条件再谈精度 |
