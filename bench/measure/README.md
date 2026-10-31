# 测量脚本（`bench/measure/`）

这里放的是量 README 那四个数字时**实际用的脚本**，以及验证窗口观感（Acrylic）的脚本。
不是"差不多"的复刻：跑出来的数就是 `docs/BENCHMARK.md` §5 记的那些。

**改脚本要同步改那份记录**，否则下一个人复现不出来，只会以为是脚本坏了。

## 为什么这些脚本长这样（每条都对应一次踩坑）

| 做法 | 不这么做会怎样 |
|---|---|
| `UseShellExecute = $false`（走 CreateProcess） | 走 Shell 会先做"发布者验证"，未签名程序可能弹「打开文件 - 安全警告」。程序还没启动就在等人点鼠标，**人的反应时间被算进冷启动** |
| 挑窗口按"**标题含 MoeWallpaper + 面积最大 + 可见**"（`lib-window.ps1`） | 开了 `transparent: true` 之后进程里多一个 15×15 辅助窗口，`Process.MainWindowHandle` 有时指向它（实测拿到 158×26 / 15×15），计时和截图全错 |
| 脚本开头 `SetProcessDPIAware()` | PowerShell 默认 DPI 不感知：`GetWindowRect` 给逻辑像素、`CopyFromScreen` 用物理像素，150% 缩放下差 110 px，截出来窗口跑到右下角 |
| 从 app 进程按 `ParentProcessId` 递归拿整棵树 | 按进程名筛会把**别人的** `msedgewebview2.exe` 算进来（本机常驻 6 个，属于 Windows 搜索宿主）；只算主进程又会小得离谱 |
| 每次启动前等整棵树退出 | 上一个实例的 WebView2 助手还占着 user-data 目录时再启动是**热启动**，报出来的"冷启动"是假的 |
| 静置 60 秒再读内存 | 启动瞬间的峰值没有意义 |
| 冷启动报中位数 + 原始值 + 首发单列 | 只报中位数会把"刚装完第一发 ~100–200 ms"这种真实现象藏起来 |
| "无图"状态用 `HTTP(S)_PROXY` 指向死端口 | 改系统代理会影响整台机器；`net.rs` 在系统代理关闭时让 reqwest 退回读环境变量，所以只挡被测进程就够 |
| 验证 Acrylic 用**整屏** `CopyFromScreen` + 窗口置顶 | `PrintWindow` 只渲染窗口自己画的东西，DWM 的 Acrylic 不在里面，截出来必然误判成"没生效"；窗口不置顶则会截到压在它上面的别的程序 |
| `.ps1` 存成 **UTF-8 带 BOM** | PowerShell 5.1 按 GBK 读，中文注释可能把换行吃掉、把下一行代码并进注释，报一个看不懂的 `Unexpected token '}'`（和 `push-retry.ps1` 一个坑） |

## 文件

| 文件 | 干什么 | 产出 |
|---|---|---|
| `lib-window.ps1` | 公共件：挑窗口 / 等窗口 / 关窗口 / 声明 DPI 感知。**别绕过它直接用 `MainWindowHandle`** | — |
| `tauri-cli-workaround.js` | 在 AI harness 里打包（那里 `node` 其实是宿主 Electron exe，CLI 会误判 argv） | 安装包与 exe |
| `measure-startup.ps1` | 冷启动：10 发，CreateProcess → 标题窗口出现，10 ms 轮询（§2.3 口径） | README「冷启动 39.5 ms」 |
| `measure-memory.ps1` | 空闲内存：整棵树 + 每个进程的角色，60 秒静置 | 有图 3 次采样（307.7 MB 中位） |
| `measure-offline.ps1` | 同一套内存口径，但把 `HTTP(S)_PROXY` 指向 `127.0.0.1:1` 挡住取图 | 无图 3 次采样（231.0 MB 中位） |
| `measure-backdrop.ps1` | 临时切亮/暗，置顶窗口后**整屏**截图 | 真 Acrylic 的目视证据 |
| `measure-theme.ps1` | 早期版本：`PrintWindow` 截窗口（只证明窗口内配色） | 主题两套配色的第一轮验证 |
| `measure-foreground.ps1` | 单次采样 + 截图，用来固定"界面当时是什么状态" | 错误态 / 有图态截图 |
| `measure-all.ps1` | 最早用的合并版（冷启动 + 内存 + 截图一条龙），第一批数据出自它 | 冷启动第 1 批、内存采样 1–3 |
| `list-windows.ps1` | 列某进程的所有顶层窗口（调试"截到错的窗口"时先跑它） | 窗口清单 |

## 跑法

```powershell
# 1) 打包（等价于 npm run tauri build；在仓库根跑）
node bench\measure\tauri-cli-workaround.js build

# 2) 装到一个空目录（NSIS 静默参数：/D= 必须是最后一个参数、不加引号）
.\src-tauri\target\release\bundle\nsis\MoeWallpaper_0.1.2_x64-setup.exe /S /D=D:\bench\install

# 3) 冷启动 / 内存（PowerShell 5.1 就行；这个环境里没有 pwsh）
powershell -NoProfile -ExecutionPolicy Bypass -File bench\measure\measure-startup.ps1 `
    -ExePath 'D:\bench\install\moe-wallpaper.exe' -Runs 10 -PollMs 10 -OutDir D:\bench\out

powershell -NoProfile -ExecutionPolicy Bypass -File bench\measure\measure-memory.ps1 `
    -ExePath 'D:\bench\install\moe-wallpaper.exe' -Samples 3 -IdleSeconds 60

powershell -NoProfile -ExecutionPolicy Bypass -File bench\measure\measure-offline.ps1 `
    -ExePath 'D:\bench\install\moe-wallpaper.exe' -Samples 3 -IdleSeconds 60

# 4) Acrylic / 主题：亮暗各截一张整屏（会临时改系统"应用主题"，结束时还原）
powershell -NoProfile -ExecutionPolicy Bypass -File bench\measure\measure-backdrop.ps1 `
    -ExePath 'D:\bench\install\moe-wallpaper.exe' -Theme light -OutDir D:\bench\out
powershell -NoProfile -ExecutionPolicy Bypass -File bench\measure\measure-backdrop.ps1 `
    -ExePath 'D:\bench\install\moe-wallpaper.exe' -Theme dark  -OutDir D:\bench\out
```

`measure-backdrop.ps1` / `measure-theme.ps1` 改的是
`HKCU\...\Themes\Personalize\AppsUseLightTheme`（玻璃主题靠 `prefers-color-scheme` 跟随它）。
**只影响视觉，不影响时序/内存数字**，但别在跑冷启动或内存测量的同时用。

## 两个必须记住的读数规则

1. **内存数字必须带界面状态。** 最新构建：无图 231.0 MB / 有图 307.7 MB（私有）。
   默认图源 360 的 `thumbnailUrl` 映射到 `img_1280_800`（真缩略图）；改之前映射的是
   `url_thumb`，那个字段实测返回原图，同样两个状态是 198.6 / 465.8——差值 267 MB 对
   77 MB，就是那一行配置的差别。
2. **私有内存不是稳定量。** 有图那格 299.3–313.2 MB、无图 215.1–238.8 MB，抖动集中
   在 `gpu-process`。报中位数 + 区间，别承诺精确到个位的数字（产品决定，见
   `docs/BENCHMARK.md` §5）。
