# 测量脚本（`bench/measure/`）

这里放的是 **2026-10-31 在 `v0.1.2` release 构建上量出那四个数字时实际用的脚本**。
不是重新写的"差不多"版本：原样复制过来，只改了 `tauri-cli-workaround.js` 里一行
写死的仓库路径（改成按当前工作目录找 CLI），其余没动。

数字和结论写在 `docs/BENCHMARK.md` §5，README 的表格从那里抄。**改脚本要同步改那份
记录**，否则下一个人复现不出来，只会以为是脚本坏了。

## 为什么这些脚本长这样（每一条都对应一次踩坑）

| 做法 | 不这么做会怎样 |
|---|---|
| `UseShellExecute = $false`（走 CreateProcess） | 走 Shell 会先做"发布者验证"，未签名程序可能弹「打开文件 - 安全警告」。程序还没启动就在等人点鼠标，**人的反应时间被算进冷启动** |
| 从 app 进程按 `ParentProcessId` 递归拿整棵树 | 按进程名筛会把**别人的** `msedgewebview2.exe` 算进来（本机常驻 6 个，属于 Windows 搜索宿主）；只算主进程又会小得离谱 |
| 每次启动前等整棵树退出 | 上一个实例的 WebView2 助手还占着 user-data 目录时再启动是**热启动**，报出来的"冷启动"是假的 |
| 静置 60 秒再读内存 | 启动瞬间的峰值没有意义 |
| 冷启动报中位数 + 原始值 + 首发单列 | 只报中位数会把"刚装完第一发 203 ms"这种真实现象藏起来 |
| "无图"状态用 `HTTP(S)_PROXY` 指向死端口 | 改系统代理会影响整台机器和别的程序；`net.rs` 在系统代理关闭时让 reqwest 退回读环境变量，所以只挡被测进程就够 |
| 截图用 `PrintWindow(..., 2)` | `CopyFromScreen` 抄的是**屏幕**：窗口没有前台焦点时截到的是压在它上面的东西（第一次就截到了编辑器和另一个浏览器页）。`SetForegroundWindow` 会被前台锁拒绝 |

## 文件

| 文件 | 干什么 | 产出 |
|---|---|---|
| `tauri-cli-workaround.js` | 在 AI harness 里打包（那里 `node` 其实是宿主 Electron exe，CLI 会误判 argv） | 安装包与 exe |
| `measure-startup.ps1` | 冷启动：10 发，CreateProcess → `MainWindowHandle` 出现，10 ms 轮询（§2.3 口径） | README「冷启动 31 ms」 |
| `measure-memory.ps1` | 空闲内存：整棵树 + 每个进程的角色，60 秒静置 | 有图 7 次采样（465.8 MB 中位） |
| `measure-offline.ps1` | 同一套内存口径，但把 `HTTP(S)_PROXY` 指向 `127.0.0.1:1` 挡住取图 | 无图 3 次采样（198.6 MB 中位） |
| `measure-foreground.ps1` | 单次采样 + `PrintWindow` 截图，用来固定"界面当时是什么状态"的证据 | 错误态/有图态的窗口截图 |
| `measure-all.ps1` | 最早用的合并版（冷启动 + 内存 + 截图一条龙），第一批数据出自它 | 冷启动第 1 批、内存采样 1–3 |

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
    -ExePath 'D:\bench\install\moe-wallpaper.exe' -Samples 2 -IdleSeconds 60
```

## 两个必须记住的读数规则

1. **内存数字必须带界面状态。** 同一构建：无图 198.6 MB / 有图 465.8 MB（私有）。
   仓库里默认图源是 360，它的 `url_thumb` 实测返回的就是 1920×1080 / 2560×1440 的
   原图（不是缩略图），所以"有图"那格等于 20 张全尺寸壁纸解码后的样子。
2. **私有内存不是稳定量。** 7 次采样私有 187.0–580.5 MB，工作集 413.7–532.8 MB，
   抖动集中在 `gpu-process`。报中位数时要写区间；想知道能不能只报一个数，先看
   `docs/BENCHMARK.md` §5 末尾那条待办。
