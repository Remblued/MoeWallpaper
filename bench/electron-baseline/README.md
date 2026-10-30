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

## 当前状态：未完成（2026-10-30）

依赖装好了（`node_modules` 约 82 MB，不入库），但打包卡在**下载工具链**：

- 不设代理：`connect ETIMEDOUT 20.205.243.166:443`（GitHub）
- 设代理：`The server aborted pending request`，下到一半断

本机当时走手机热点 + 代理，下载 200 MB 级资源不稳定。**重试只需跑最后一条命令**，
不需要重装依赖。

README 里的 Electron 基线因此保持 `TBD`。
