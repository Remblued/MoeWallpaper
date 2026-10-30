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

**冷启动和空闲内存仍是 `TBD`**：这两项要求启动 GUI 进程，需要在正常的桌面会话里
人工跑 `docs/BENCHMARK.md` §3.2。
