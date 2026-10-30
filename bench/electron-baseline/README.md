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

**冷启动和空闲内存仍是 `TBD`**：这两项要求启动 GUI 进程，需要在正常的桌面会话里跑
`measure.ps1`（口径见 `docs/BENCHMARK.md` §2.3 / §2.4）：

```powershell
cd bench\electron-baseline
npx electron-builder --win nsis      # 改过 main.js 就必须重打包
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
